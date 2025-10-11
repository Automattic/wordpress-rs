# Nav Menu Item Autosaves Investigation

## Overview

This document investigates the behavior of the WordPress REST API autosaves endpoint for nav menu items (`/wp/v2/menu-items/<id>/autosaves`). During integration test implementation, we discovered that autosaves for nav menu items behave differently than autosaves for posts and pages.

## Controller Architecture

The nav menu items autosaves endpoint is handled by:
1. **WP_REST_Autosaves_Controller** - Generic autosaves controller at `wp-includes/rest-api/endpoints/class-wp-rest-autosaves-controller.php`
2. **WP_REST_Menu_Items_Controller** - Nav menu items controller at `wp-includes/rest-api/endpoints/class-wp-rest-menu-items-controller.php` (extends `WP_REST_Posts_Controller`)

### How Autosaves Work

From `class-wp-rest-autosaves-controller.php` line 223:
```php
$prepared_post = $this->parent_controller->prepare_item_for_database( $request );
```

The autosaves controller delegates to the parent controller's `prepare_item_for_database` method to prepare the data before creating the autosave revision.

## Problem: Fields Not Being Saved

When creating an autosave for a nav menu item, **none of the content fields are being saved** to the autosave revision. This includes:
- `title`
- `url`
- `description`
- `attr_title`
- `classes`
- `xfn`
- etc.

### Test Commands

All commands below can be copy-pasted and run directly - they fetch credentials from `test_credentials.json` automatically.

#### Get the nav menu item details:
```bash
curl --silent --user "$(jq -r '.admin_username' test_credentials.json):$(jq -r '.admin_password' test_credentials.json)" \
  "http://localhost/wp-json/wp/v2/menu-items/$(jq -r '.nav_menu_item_id' test_credentials.json)" | jq '{id, title, type, url, status: .status}'
```

Expected output:
```json
{
  "id": 1890,
  "title": {
    "rendered": "Integration Test Nav Menu Item"
  },
  "type": "custom",
  "url": "https://example.com",
  "status": "publish"
}
```

#### Create autosave with title only:
```bash
curl --silent --user "$(jq -r '.admin_username' test_credentials.json):$(jq -r '.admin_password' test_credentials.json)" \
  -H "Content-Type: application/json" \
  -d '{"title":"Test Autosave Title"}' \
  "http://localhost/wp-json/wp/v2/menu-items/$(jq -r '.nav_menu_item_id' test_credentials.json)/autosaves" | jq '{id, title}'
```

Actual output:
```json
{
  "id": 1891,
  "title": {
    "raw": "",
    "rendered": ""
  }
}
```

**Problem**: Title is empty despite being provided in the request.

#### Create autosave with title and description:
```bash
curl --silent --user "$(jq -r '.admin_username' test_credentials.json):$(jq -r '.admin_password' test_credentials.json)" \
  -H "Content-Type: application/json" \
  -d '{"title":"Test Title","description":"Test Description"}' \
  "http://localhost/wp-json/wp/v2/menu-items/$(jq -r '.nav_menu_item_id' test_credentials.json)/autosaves" | jq '{id, title, description}'
```

Actual output:
```json
{
  "id": 1891,
  "title": {
    "raw": "",
    "rendered": ""
  },
  "description": null
}
```

**Problem**: Both title and description are empty/null.

#### Create autosave with title and URL (required for custom type):
```bash
curl --silent --user "$(jq -r '.admin_username' test_credentials.json):$(jq -r '.admin_password' test_credentials.json)" \
  -H "Content-Type: application/json" \
  -d '{"title":"Autosave Title","url":"https://autosave.example.com"}' \
  "http://localhost/wp-json/wp/v2/menu-items/$(jq -r '.nav_menu_item_id' test_credentials.json)/autosaves" | jq '{id, title, url}'
```

Actual output:
```json
{
  "id": 1891,
  "title": {
    "raw": "",
    "rendered": ""
  },
  "url": null
}
```

**Problem**: Both title and URL are empty/null despite being required fields for custom menu items.

#### List all autosaves:
```bash
curl --silent --user "$(jq -r '.admin_username' test_credentials.json):$(jq -r '.admin_password' test_credentials.json)" \
  "http://localhost/wp-json/wp/v2/menu-items/$(jq -r '.nav_menu_item_id' test_credentials.json)/autosaves" | jq
```

Output shows autosave exists but with empty content:
```json
[
  {
    "author": 1,
    "date": "2025-10-01T00:31:10",
    "date_gmt": "2025-10-01T00:31:10",
    "id": 1891,
    "modified": "2025-10-01T00:31:10",
    "modified_gmt": "2025-10-01T00:31:10",
    "parent": 1890,
    "slug": "1890-autosave-v1",
    "guid": {
      "rendered": "http://localhost/?p=1891",
      "raw": "http://localhost/?p=1891"
    },
    "title": {
      "rendered": ""
    },
    "meta": [],
    "_links": {
      "parent": [
        {
          "href": "http://localhost/wp-json/wp/v2/menu-items/1890"
        }
      ]
    }
  }
]
```

## Root Cause Analysis - Deep Dive into WordPress Source Code

### The Problem: Data Format Mismatch

After comprehensive source code analysis, we've identified the exact reason why nav menu item autosaves don't save any content fields. The issue is a **fundamental incompatibility** between how menu items store data and how the WordPress revision system works.

### Menu Items Controller `prepare_item_for_database`

The `WP_REST_Menu_Items_Controller::prepare_item_for_database()` method has special handling for nav menu items (lines 343-492 in `class-wp-rest-menu-items-controller.php`):

1. **Line 344**: It expects `$request['id']` to be the menu item database ID:
```php
$menu_item_db_id = $request['id'];
```

2. **Line 345**: It retrieves the existing menu item object:
```php
$menu_item_obj = $this->get_nav_menu_item( $menu_item_db_id );
```

3. **Lines 347-368**: If the menu item exists, it populates `$prepared_nav_item` with the **current values** from the database:
```php
$prepared_nav_item = array(
    'menu-item-db-id'       => $menu_item_db_id,
    'menu-item-object-id'   => $menu_item_obj->object_id,
    'menu-item-object'      => $menu_item_obj->object,
    // ... all fields from existing object
    'menu-item-title'       => $menu_item_obj->title,
    'menu-item-url'         => $menu_item_obj->url,
    // ...
);
```

4. **Lines 407-411**: Then it applies values from the request **over** the prepared data:
```php
foreach ( $mapping as $original => $api_request ) {
    if ( isset( $request[ $api_request ] ) ) {
        $prepared_nav_item[ $original ] = $request[ $api_request ];
    }
}
```

5. **Lines 421-427**: Special handling for title field:
```php
if ( ! empty( $schema['properties']['title'] ) && isset( $request['title'] ) ) {
    if ( is_string( $request['title'] ) ) {
        $prepared_nav_item['menu-item-title'] = $request['title'];
    } elseif ( ! empty( $request['title']['raw'] ) ) {
        $prepared_nav_item['menu-item-title'] = $request['title']['raw'];
    }
}
```

### The Autosave Problem: Three-Layer Data Transformation

When the autosaves controller creates an autosave for a menu item, the data goes through three incompatible layers:

#### Layer 1: Menu Items Controller Returns Menu-Item Format

`WP_REST_Menu_Items_Controller::prepare_item_for_database()` returns an object with menu-item-specific properties:

```php
$prepared_nav_item = (object) array(
    'menu-item-title'       => 'Test Title',  // ← Menu item format
    'menu-item-url'         => 'https://example.com',
    'menu-item-description' => 'Test Description',
    'menu-item-attr-title'  => 'Test Attr',
    // ... other menu-item-* fields
);
```

This format is designed for `wp_update_nav_menu_item()`, which knows how to convert these special fields into post meta.

#### Layer 2: Autosaves Controller Passes to Revision System

The autosaves controller (`class-wp-rest-autosaves-controller.php:223`) calls:

```php
$prepared_post = $this->parent_controller->prepare_item_for_database( $request );
```

Then passes this directly to the revision system (`line 243`):

```php
$autosave_id = $this->create_post_autosave( (array) $prepared_post, (array) $request->get_param( 'meta' ) );
```

#### Layer 3: Revision System Only Understands Standard Post Fields

The `create_post_autosave()` method calls `_wp_post_revision_data()` (`class-wp-rest-autosaves-controller.php:377`):

```php
$new_autosave = _wp_post_revision_data( $post_data, true );
```

This function (`wp-includes/revision.php:75-96`) only extracts fields that match `_wp_post_revision_fields()`:

```php
function _wp_post_revision_fields( $post = array(), $deprecated = false ) {
    $fields = array(
        'post_title'   => __( 'Title' ),      // ← Looking for post_title
        'post_content' => __( 'Content' ),    // ← Looking for post_content
        'post_excerpt' => __( 'Excerpt' ),    // ← Looking for post_excerpt
    );
    // ...
    return $fields;
}
```

```php
function _wp_post_revision_data( $post = array(), $autosave = false ) {
    $fields = _wp_post_revision_fields( $post );
    $revision_data = array();

    // Only copies fields that exist in BOTH $post and $fields
    foreach ( array_intersect( array_keys( $post ), array_keys( $fields ) ) as $field ) {
        $revision_data[ $field ] = $post[ $field ];
    }
    // ...
}
```

### The Fatal Mismatch

The object has properties like:
- `menu-item-title`
- `menu-item-url`
- `menu-item-description`

But `_wp_post_revision_data()` is looking for:
- `post_title`
- `post_content`
- `post_excerpt`

**Result**: `array_intersect()` finds NO matching keys, so `$revision_data` gets ZERO content fields. The autosave is created with empty content.

### Why This Works for Regular Posts But Not Menu Items

**For regular posts/pages:**
1. `prepare_item_for_database()` returns a `WP_Post` object with standard fields:
   ```php
   $post = (object) array(
       'post_title'   => 'My Title',    // ✅ Matches revision field
       'post_content' => 'My Content',  // ✅ Matches revision field
       'post_excerpt' => 'My Excerpt',  // ✅ Matches revision field
   );
   ```
2. `_wp_post_revision_data()` finds these matching keys
3. Autosave is created with full content

**For nav menu items:**
1. `prepare_item_for_database()` returns an object with menu-specific fields:
   ```php
   $nav_item = (object) array(
       'menu-item-title' => 'My Title',  // ❌ No match
       'menu-item-url'   => 'https://example.com',  // ❌ No match
       // ... all other menu-item-* fields ❌ No match
   );
   ```
2. `_wp_post_revision_data()` finds ZERO matching keys
3. Autosave is created completely empty

### Why Menu Items Use a Special Format

Nav menu items are stored as `nav_menu_item` post type, but their actual content lives in post meta fields:
- `_menu_item_title` - The display title
- `_menu_item_url` - The URL
- `_menu_item_description` - The description
- `_menu_item_target` - Link target (_blank, etc.)
- `_menu_item_classes` - CSS classes
- `_menu_item_xfn` - XFN relationship
- etc.

The special `menu-item-*` format is used by `wp_update_nav_menu_item()` which knows how to convert these into the appropriate post meta fields. However, the autosave/revision system never calls `wp_update_nav_menu_item()` - it goes directly to the low-level revision functions which don't understand this format.

## Comprehensive Field Testing

### Test Script

A comprehensive test script was created at `test_nav_menu_item_autosave_fields.sh` that systematically tests all fields documented in the WordPress REST API documentation for creating nav menu item autosaves.

The script tests each field individually by:
1. Creating an autosave with only that field set
2. Retrieving the autosave to verify if the field was saved
3. Restoring the server between tests using `make restore-server`

### Test Results

**All 13 content fields FAILED to save:**

| Field | Expected Value | Actual Value | Result |
|-------|---------------|--------------|--------|
| `title` | "TEST AUTOSAVE TITLE" | `""` (empty) | ❌ NOT saved |
| `type` | "post_type" | `null` | ❌ NOT saved |
| `status` | "draft" | `null` | ❌ NOT saved |
| `attr_title` | "Test Attribute Title" | `null` | ❌ NOT saved |
| `classes` | `["test-class-1","test-class-2"]` | `null` | ❌ NOT saved |
| `description` | "Test Description" | `null` | ❌ NOT saved |
| `menu_order` | `5` | `null` | ❌ NOT saved |
| `object` | "page" | `null` | ❌ NOT saved |
| `object_id` | `999` | `null` | ❌ NOT saved |
| `target` | "_blank" | `null` | ❌ NOT saved |
| `url` | "https://test-autosave.example.com" | `null` | ❌ NOT saved |
| `xfn` | `["friend","colleague"]` | `null` | ❌ NOT saved |
| `meta` | `{}` | `[]` | ❌ NOT saved |

**One field was rejected as invalid:**

| Field | Result |
|-------|--------|
| `menus` | API error: "Invalid parameter(s): menus" |

### Conclusion

The comprehensive testing confirms that **none of the documented fields for creating nav menu item autosaves actually work**. The API accepts the requests (returns 201 Created for most fields), but completely ignores all field values. The autosaves are created but contain no content from the request.

## Workaround Status

### Current Test Implementation

The integration test in `test_nav_menu_item_autosaves_mut.rs` was updated to only verify:
1. An autosave can be created (returns 200 OK)
2. The autosave has an ID
3. The autosave references the correct parent menu item

**We do NOT verify that content fields are saved**, since comprehensive testing proves they are not saved regardless of what's sent in the request.

## Conclusion: This is a WordPress Core Bug

Based on the comprehensive source code analysis, **this is definitively a bug in WordPress core**. The generic autosaves controller (`WP_REST_Autosaves_Controller`) was designed to work with standard post types (posts, pages) that store content in `post_title`, `post_content`, and `post_excerpt` fields.

When support for nav menu item autosaves was added (likely in WordPress 5.9 when nav menu items got REST API support), the autosaves endpoint was automatically exposed for nav menu items through the generic controller. However, **nobody adapted the autosave logic to handle the special `menu-item-*` data format**.

### The Bug

The bug exists in `WP_REST_Autosaves_Controller::create_post_autosave()` which assumes all post types use standard post fields. It should have special handling for nav menu items (similar to how `wp_update_nav_menu_item()` handles them), but it doesn't.

### Impact

1. ✅ The endpoint exists and accepts requests
2. ✅ Autosaves are created (they get an ID and are stored)
3. ❌ All content fields are silently ignored
4. ❌ The autosaves are completely empty
5. ❌ The API returns 201 Created, making it appear successful

This is a **silent data loss bug** - the API reports success but loses all the data.

## Recommendations

1. **WordPress Core Ticket**:
   - This should be reported to WordPress core as a bug
   - The fix would require the autosaves controller to detect nav menu items and either:
     - Option A: Call `wp_update_nav_menu_item()` instead of the generic revision functions
     - Option B: Convert `menu-item-*` fields to standard post fields before passing to revision system
     - Option C: Add `wp_post_revision_meta_keys` filter support for `_menu_item_*` meta fields
     - Option D: Disable autosaves entirely for nav menu items (if they're not useful)

2. **Library Documentation**:
   - Document this limitation clearly in the Rust library
   - Add warnings that nav menu item autosaves don't work
   - Consider whether to even expose the autosave endpoints for menu items

3. **Testing Strategy**:
   - Keep the basic "autosave can be created" test (to verify endpoint availability)
   - Add explicit tests that document fields are NOT saved (to detect if WordPress behavior changes)
   - Reference this investigation document in test comments

4. **Workaround Status**:
   - There is no workaround at the library level
   - This must be fixed in WordPress core
   - Until fixed, nav menu item autosaves are essentially non-functional

## References

- WordPress REST API Autosaves Controller: `wp-includes/rest-api/endpoints/class-wp-rest-autosaves-controller.php`
- WordPress REST API Menu Items Controller: `wp-includes/rest-api/endpoints/class-wp-rest-menu-items-controller.php`
- WordPress Revisions: `wp-includes/revision.php` (`_wp_post_revision_fields()`, `_wp_post_revision_data()`)
- Nav Menu Items Storage: Menu item data is stored in post meta fields with `_menu_item_*` keys
