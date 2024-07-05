# WP REST API Project CHANGELOG

---

## Trunk

### Breaking Changes

_None_

### New Features

_None_

### Bug Fixes

_None_

### Internal Changes

_None_

## 0.1

This is the first release. It supports the following for the Swift and Kotlin platforms:
- Application Token-based login
- Application Token Management endpoints
- Users endpoints
- Plugins endpoints

It also includes all of the underlying infrastructure for this – including:
- `wp_contextual` – a set of macros for generating model objects from a DSL that closely matches https://developer.wordpress.org/rest-api/reference/.
- `wp_derive_request_builder` – a set of macros for generating endpoint implementations based on Rust's strong generic type support.
