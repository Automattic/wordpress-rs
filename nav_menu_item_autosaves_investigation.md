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

## Root Cause Analysis

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

### The Autosave Problem

When the autosaves controller calls `prepare_item_for_database()`:

1. The `$request['id']` contains the **parent menu item ID** (e.g., 1890)
2. The method retrieves the parent menu item and uses its current values
3. Request fields like `title`, `url`, etc. **should** be applied to override the parent values
4. However, the autosave is being created with **empty/null values** instead

### Hypothesis

The issue likely occurs in how WordPress's revision system handles nav menu items. After `prepare_item_for_database()` returns the prepared data, the autosaves controller calls `create_post_autosave()` (line 243):

```php
$autosave_id = $this->create_post_autosave( (array) $prepared_post, (array) $request->get_param( 'meta' ) );
```

This method uses `_wp_post_revision_data()` to determine which fields should be included in the revision. Nav menu items may have different revisioned fields than regular posts, or the conversion from the menu-item-specific array format to the post revision format may be losing data.

## WordPress Post Revision Fields

The `_wp_post_revision_fields()` function determines which fields are stored in revisions. For nav menu items (which are stored as `nav_menu_item` post type), the revisioned fields are likely limited to the standard post fields:
- `post_title`
- `post_content`
- `post_excerpt`

However, nav menu items store their actual data in **post meta**, not in these standard post fields:
- Title is stored in `_menu_item_title` meta
- URL is stored in `_menu_item_url` meta
- Description is stored in `_menu_item_description` meta
- etc.

**This is likely why autosaves appear empty** - the WordPress revision system doesn't include the menu item meta fields in revisions by default.

## Comparison with Posts Autosaves

For posts, autosaves work correctly because:
1. Posts store content in standard fields (`post_title`, `post_content`, `post_excerpt`)
2. These fields are included in `_wp_post_revision_fields()`
3. The autosave captures these changes

For nav menu items:
1. Menu items store content in **meta fields** (not standard post fields)
2. Meta fields may not be included in the default revisioned fields
3. The autosave is created but doesn't capture the menu-specific data

## Workaround Status

### Current Test Implementation

The integration test in `test_nav_menu_item_autosaves_mut.rs` was updated to only verify:
1. An autosave can be created (returns 200 OK)
2. The autosave has an ID
3. The autosave references the correct parent menu item

**We do NOT verify that content fields are saved**, since they appear to be empty regardless of what's sent in the request.

## Recommendations

1. **WordPress Core Issue**: This may be a limitation or bug in WordPress core. Nav menu items might not properly support autosaves with revisioned content.

2. **Further Investigation Needed**:
   - Check if `wp_post_revision_meta_keys` filter is used to add menu item meta fields to revisions
   - Test if this behavior is consistent across different WordPress versions
   - Determine if this is expected behavior or a WordPress bug

3. **Potential Solutions**:
   - File a WordPress core ticket if this is determined to be a bug
   - Document this limitation in the library
   - Consider if nav menu item autosaves are even useful without content preservation

4. **Testing Strategy**:
   - Keep the basic "autosave can be created" test
   - Add a comment explaining the limitation
   - Consider adding a test that explicitly verifies fields are NOT saved (to detect if WordPress behavior changes)

## References

- WordPress REST API Autosaves Controller: `wp-includes/rest-api/endpoints/class-wp-rest-autosaves-controller.php`
- WordPress REST API Menu Items Controller: `wp-includes/rest-api/endpoints/class-wp-rest-menu-items-controller.php`
- WordPress Revisions: `wp-includes/revision.php` (`_wp_post_revision_fields()`, `_wp_post_revision_data()`)
- Nav Menu Items Storage: Menu item data is stored in post meta fields with `_menu_item_*` keys
