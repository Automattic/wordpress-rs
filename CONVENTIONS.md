# Conventions

Rules for modelling the WordPress and WordPress.com REST APIs in this crate. They exist because the same decision keeps coming up on every new endpoint, and because getting one wrong usually costs more than the field it affects — a mistyped field fails the whole response, not just itself.

## Dates

Every field holding a date or a time is one of two types. Never a bare `String`.

**`WpGmtDateTime`** — the value resolves to an absolute instant, because it is UTC or carries an offset. The bindings lower it to a unix timestamp, so a value that isn't genuinely an instant becomes a wrong one.

**`WpDateString`** — the value denotes a date but can't be resolved to an instant. Two common shapes: a bare calendar date (`2026-08-06`), and a datetime in the *site's* timezone (`2026-08-06 09:15:49`), which needs the site's offset to place. It carries the API's string unchanged.

Where an endpoint sends both a local and a GMT form of the same timestamp, model both — the GMT one as `WpGmtDateTime`, its local twin as `WpDateString`.

### The field name doesn't settle it

Check what the endpoint implementation actually produces before choosing. Fields that read as dates and aren't, and fields that don't and are, have both turned up:

- A time-series `period` is a **label**, not a date. Its shape follows the unit the caller requested, so a weekly series returns `2026W02W23` and a yearly one returns `2024`. Only the daily form looks like a date.
- A `period` on a summarised stats response is the grouping **unit** — `"day"`, `"week"` — rather than any kind of date.
- A "most active day" is derived from a comment's **GMT** timestamp, so it is an instant and takes `WpGmtDateTime`, even though the field reports a day and the time of day it carries is incidental.
- "Humanized" fields (`humanized_updated`, and similar) are prose for display.
- Date *format* settings, timezone names and bill-period lengths are configuration, not dates.

### Absent values

Some endpoints send boolean `false` rather than `null` when a date doesn't apply. `deserialize_optional_date_string` handles that for `WpDateString`. For `WpGmtDateTime`, `deserialize_optional_wp_gmt_date_time` treats `null` and `""` as absent.
