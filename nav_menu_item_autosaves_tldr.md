# Nav Menu Item Autosaves - TL;DR

## The Problem

**None of the fields work when creating nav menu item autosaves via the WordPress REST API.**

The endpoint `/wp/v2/menu-items/<id>/autosaves` accepts requests and returns `201 Created`, but **all content fields are silently ignored**. The autosaves are created completely empty.

## Test Results

We tested all 14 fields documented in the API:

| Status | Fields |
|--------|--------|
| ❌ **All Failed** | `title`, `type`, `status`, `attr_title`, `classes`, `description`, `menu_order`, `object`, `object_id`, `target`, `url`, `xfn`, `meta` |
| ❌ **Rejected** | `menus` (returns error: "Invalid parameter(s)") |

**Result**: 0 out of 14 fields work. All return `null` or empty values.

## Root Cause

This is a **WordPress core bug** caused by a data format mismatch in the autosave flow:

```
Menu Items Controller → Autosaves Controller → Revision System
   (menu-item-* format)                        (expects post_title, post_content, post_excerpt)
         ↓                                                    ↓
   Returns object with:                              Looking for:
   • menu-item-title                                 • post_title
   • menu-item-url                                   • post_content
   • menu-item-description                           • post_excerpt

   Result: array_intersect() finds ZERO matches → All data is lost
```

**The bug**: `WP_REST_Autosaves_Controller::create_post_autosave()` calls `_wp_post_revision_data()` which only extracts standard post fields. It has no special handling for nav menu items' unique `menu-item-*` format.

**The impact**: Silent data loss. The API reports success but saves nothing.

## Why This Matters

- **For API consumers**: You can't trust this endpoint. It appears to work but loses all your data.
- **For WordPress**: This endpoint was auto-exposed when menu items got REST API support (WordPress 5.9), but nobody adapted the autosave logic for menu items' special data format.
- **For our library**: We can only verify the endpoint exists. We cannot verify content is saved because it isn't.

## What to Do

1. **Report to WordPress core** - This needs to be fixed upstream
2. **Document the limitation** - Warn users that this endpoint is non-functional
3. **Test script available** - Run `./test_nav_menu_item_autosave_fields.sh` to reproduce

## Full Analysis

See `nav_menu_item_autosaves_investigation.md` for:
- Complete source code analysis with line numbers
- Detailed explanation of the three-layer data transformation
- Test methodology and curl commands
- Potential fix options for WordPress core
