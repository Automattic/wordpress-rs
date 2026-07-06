---
name: buildkite-triage
description: Extract the failing tests and steps from a wordpress-rs Buildkite build via the Buildkite MCP, without downloading full logs. Use when asked to triage, diagnose, or summarize a failing CI build or job — e.g. given a buildkite.com/automattic/wordpress-rs/builds/NNNN URL or build number — or to find which tests failed and why.
---

# Buildkite Failure Triage (wordpress-rs)

Pull only the failures from a Buildkite build: structured test failures where a collector exists (Kotlin), and targeted log search everywhere else (Rust, Swift). The Rust integration log alone is 100k+ lines — never scan it whole.

## Requirement: Buildkite MCP (check this first)

This skill **requires the Buildkite MCP server to be enabled and authorized**. Before doing anything else, load every tool you'll need in one `ToolSearch` call to avoid round-trips: `select:mcp__buildkite__list_jobs,mcp__buildkite__get_build,mcp__buildkite__get_build_test_engine_runs,mcp__buildkite__get_failed_executions,mcp__buildkite__search_logs,mcp__buildkite__read_logs`. One of three things is true:

- **The tools load** → the server is authorized; proceed.
- **Only `mcp__buildkite__authenticate` is available** (server connected but not yet authorized — the full toolset is hidden until then) → run it, have the user complete the OAuth flow in the browser, then load the tools and proceed.
- **No `mcp__buildkite__*` tools exist at all** → **STOP** and tell the user, verbatim:

  > This skill requires the Buildkite MCP to be enabled and authorized. Configure it per https://buildkite.com/docs/apis/mcp-server, then re-run.

If an already-authorized call later returns an authorization error, treat it as the "not authorized" case and STOP with the same message rather than guessing. Do **not** fall back to scraping the Buildkite website, `curl`, or the public API — if the MCP is unavailable, stop.

## Coordinates

- **org_slug**: `automattic`
- **pipeline_slug**: `wordpress-rs`
- **build_number**: from the user. A URL like `https://buildkite.com/automattic/wordpress-rs/builds/5776` → `5776`.

## Step 1 — Find the failed jobs

`list_jobs` with `state: "failed,broken"`, `per_page: 100`. Record each failed job's `name`, `id`, and `exit_status`.

- `state: "failed"` = a step actually failed. `state: "broken"` = skipped by a dependency or branch condition; usually not the root cause, so note but don't chase it.
- Match the job `name` to the language: `:rust:` / `:kotlin:` / `:swift:` prefixes tell you which step below to use.
- Steps 2–4 for different failed jobs are independent — issue their calls concurrently rather than one job at a time.

## Step 2 — Kotlin failures (has a collector)

The Kotlin integration step uploads JUnit XML to Buildkite Test Engine, so its failures come back structured:

1. `get_build_test_engine_runs` → returns an **array** of runs. Pick the failing one; take its `run_id` and `suite.slug` (currently always `wordpress-rs`). Shortcut: if you already called `get_build` (e.g. for branch context in the Report), it embeds `test_engine.runs[]` with the same `id`/`suite.slug` — reuse those and skip this call.
2. `get_failed_executions` with `run_id`, `test_suite_slug: "wordpress-rs"` (the parameter is named `test_suite_slug`; its value is the `suite.slug` from step 1), and `include_failure_expanded: true` → failing test names, assertion messages, and stack traces.

This is the only suite in Test Engine (see the note at the end). Do not expect Rust or Swift tests here.

## Step 3 — Rust / Swift failures (no collector)

libtest and XCTest don't feed Test Engine, so extract failures from the failed job's log with a targeted search — never read the whole log:

1. `search_logs` on that specific `job_id`:
   - `pattern`: `panicked at|\.\.\. FAILED|test result: FAILED`
   - `limit`: `200`
   Use `test result: FAILED`, **not** a bare `test result:` — a large suite prints one `test result: ok` line per passing binary, and those will exhaust the match budget before the search reaches the panic locations and the failing summary you actually need. Log lines are timestamp-prefixed, so avoid `^`-anchored patterns. This returns the failing test names (`... FAILED`), each panic location (`panicked at <file>:<line>`), and the failing binaries' summaries (`test result: FAILED. X passed; Y failed; ...`).
2. For one failure's detail: `search_logs` with `pattern` set to the test name and `after_context: 20`, or `read_logs` with `seek` at the `row_number` from step 1.

## Step 4 — Non-test failed jobs (build, publish, lint)

Some `failed` jobs aren't test runs (e.g. `:kotlin: Publish rs.wordpress.api:android`, xcframework assembly, lint). They have no collector and no libtest summary, so search their log for the error directly:

- `search_logs` on the `job_id` with `pattern: "error\[|error:|FAILED|Exception|exit status [1-9]"`, `limit: 50`, `before_context: 3`.
- Report the failing command and the error line — these are build/tooling failures, not test regressions.

## Report

First, for context, note the build's branch and message (from `get_failed_executions`' `branch`/`run_name`, or `get_build`). A dependency-bump branch (e.g. Dependabot) whose only failures are external-API errors is a strong "not caused by this PR" signal.

Then summarize per failed job:
- The failing test names (collapse large parameterized families, e.g. `filter::filter_list_with_*_context::case_*`, into a count). Attribute each to the binary whose summary is `test result: FAILED` — the same `case_*` name can exist in more than one binary, so don't assume a name maps to a single suite.
- The shared panic/assert location if there is one (e.g. all at `wp_api_integration_tests/src/lib.rs:234`).
- A one-line likely cause. Distinguish **external-dependency flakiness** from a **real regression** — the former is not a code problem.
  - Flakiness signals: an identical panic/error message across a whole external-API test family (e.g. wordpress.org pattern-directory HTTP 500) while every other suite in the build is green.
  - Regression signals: failures concentrated in the area the branch touched, varied messages, or assertion failures rather than request errors.

## Why only Kotlin has structured failures

Gradle emits JUnit XML natively, which the `test-collector` plugin uploads (`.buildkite/pipeline.yml`, Kotlin e2e step). `cargo test` cannot emit JUnit on stable Rust, and cargo-nextest — which can — is a poor fit here: the `_mut` integration tests restore the server from a backup between each test and rely on `serial_test` plus cargo's sequential test binaries for whole-suite exclusivity, which nextest's process-per-test model breaks. So for Rust, targeted log search is the intended triage path, not a collector.
