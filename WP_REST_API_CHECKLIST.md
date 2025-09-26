# WordPress REST API Simple Checklist

## Core Content Endpoints

- [x] `/wp/v2/posts`
- [x] `/wp/v2/pages`
- [x] `/wp/v2/comments`
- [x] `/wp/v2/media`
- [x] `/wp/v2/users`
- [x] `/wp/v2/categories`
- [x] `/wp/v2/tags`
- [x] `/wp/v2/taxonomies`
- [x] `/wp/v2/types`
- [x] `/wp/v2/search`
- [x] `/wp/v2/settings`
- [x] `/wp/v2/themes`
- [x] `/wp/v2/plugins`
- [ ] `/wp/v2/statuses`

## Revision & Autosave Endpoints

- [x] `/wp/v2/posts/<id>/revisions`
- [x] `/wp/v2/pages/<id>/revisions`
- [x] `/wp/v2/posts/<id>/autosaves`
- [x] `/wp/v2/pages/<id>/autosaves`

## Block Editor Endpoints

- [ ] `/wp/v2/block-types`
- [ ] `/wp/v2/blocks`
- [ ] `/wp/v2/blocks/<id>/revisions`
- [ ] `/wp/v2/blocks/<id>/autosaves`
- [ ] `/wp/v2/block-renderer/<id>`
- [ ] `/wp/v2/block-directory/search`

## Block Patterns & Templates

- [ ] `/wp/v2/block-patterns/patterns`
- [ ] `/wp/v2/block-patterns/categories`
- [ ] `/wp/v2/pattern-directory/patterns`
- [x] `/wp/v2/templates`
- [ ] `/wp/v2/templates/<id>/revisions`
- [ ] `/wp/v2/templates/<id>/autosaves`
- [ ] `/wp/v2/template-parts`
- [ ] `/wp/v2/template-parts/<id>/revisions`
- [ ] `/wp/v2/template-parts/<id>/autosaves`

## Navigation & Menus

- [ ] `/wp/v2/navigation`
- [ ] `/wp/v2/navigation/<id>/revisions`
- [ ] `/wp/v2/navigation/<id>/autosaves`
- [ ] `/wp/v2/menus`
- [ ] `/wp/v2/menu-items`
- [ ] `/wp/v2/menu-items/<id>/autosaves`
- [ ] `/wp/v2/menu-locations`

## Widgets & Sidebars

- [x] `/wp/v2/widgets`
- [x] `/wp/v2/widget-types`
- [ ] `/wp/v2/sidebars`

## Global Styles

- [ ] `/wp/v2/global-styles/<id>`
- [ ] `/wp/v2/global-styles/<id>/revisions`

## Additional Implemented Endpoints

- [x] `/wp/v2/users/<user_id>/application-passwords`
- [x] `/wp-block-editor/v1/settings`
- [ ] `/wp-block-editor/v1/navigation-fallback`
- [x] `/wp-site-health/v1/tests`
- [x] `/wp/v2` (API root)
