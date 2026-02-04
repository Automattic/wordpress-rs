# wp_com_e2e

End-to-end test runner for WordPress.com REST API endpoints.

## Overview

This module provides a test harness for validating WordPress.com API functionality. It uses [libtest-mimic](https://crates.io/crates/libtest-mimic) to provide a familiar cargo-test-like CLI experience with filtering, listing, and selective test execution.

## Setup

Set the `WP_COM_API_KEY` environment variable with a valid WordPress.com OAuth2 bearer token:

```bash
export WP_COM_API_KEY=your_token_here
```

## Usage

### Run read-only tests (default)

```bash
cargo run -p wp_com_e2e
```

By default, only read-only tests run. Write tests (creating conversations, tickets, etc.) are marked as "ignored" and skipped.

### Run all tests including writes

```bash
cargo run -p wp_com_e2e -- --include-ignored
```

### Run only write tests

```bash
cargo run -p wp_com_e2e -- --ignored
```

### List all tests

```bash
cargo run -p wp_com_e2e -- --list
```

### Filter tests by name

```bash
# Run all sites tests
cargo run -p wp_com_e2e -- sites

# Run a specific test
cargo run -p wp_com_e2e -- "me::get_user_info"

# Run tests matching a pattern
cargo run -p wp_com_e2e -- support_bots
```

### Other options

```bash
# Show help
cargo run -p wp_com_e2e -- --help

# Run tests with exact name match
cargo run -p wp_com_e2e -- --exact "me::get_user_info"

# Disable output capture (show test output)
cargo run -p wp_com_e2e -- --nocapture
```

## Test Modules

| Module | Description | Write Tests |
|--------|-------------|-------------|
| `me_tests` | Current user info and OAuth2 token validation | No |
| `sites_tests` | Site listing and retrieval by ID/slug | No |
| `support_bot_tests` | Bot conversation listing and retrieval | Yes |
| `support_eligibility_test` | Support eligibility check | No |
| `support_tickets_test` | Support ticket listing and retrieval | Yes |

## Dynamic Tests

Some test modules generate tests dynamically based on API responses:

- **sites_tests**: Creates a test for each site in the user's account
- **support_bot_tests**: Creates a test for each existing bot conversation
- **support_tickets_test**: Creates a test for each existing support conversation

## Write Tests

Write tests are marked with the "ignored" flag and include:

- `support_bots::create_conversation` - Creates a new bot conversation
- `support_bots::create_and_add_message` - Creates a conversation and adds a message
- `support_tickets::create_ticket` - Creates a new support ticket
- `support_tickets::create_and_add_message` - Creates a ticket and adds a message

These tests create real data in the WordPress.com system and should be run sparingly.
