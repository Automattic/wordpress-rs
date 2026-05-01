# WordPress REST API Simple Checklist

## Core Content Endpoints

- [x] `/wp/v2/types`
- [x] `/wp/v2/taxonomies`
- [x] `/wp/v2/statuses`

- [x] `/wp/v2/$postType`
- [x] `/wp/v2/$postType/<id>/revisions`
- [x] `/wp/v2/$postType/<id>/autosaves`

- [x] `/wp/v2/$term`
- [x] `/wp/v2/media`
- [x] `/wp/v2/comments`
- [x] `/wp/v2/search`

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
- [x] `/wp/v2/templates/<id>/revisions`
- [x] `/wp/v2/templates/<id>/autosaves`
- [x] `/wp/v2/template-parts`
- [ ] `/wp/v2/template-parts/<id>/revisions`
- [ ] `/wp/v2/template-parts/<id>/autosaves`

## Navigation & Menus

- [x] `/wp/v2/navigation`
- [x] `/wp/v2/navigation/<id>/revisions`
- [x] `/wp/v2/navigation/<id>/autosaves`
- [x] `/wp/v2/menus`
- [x] `/wp/v2/menu-items`
- [x] `/wp/v2/menu-items/<id>/autosaves`
- [x] `/wp/v2/menu-locations`

## Widgets & Sidebars

- [x] `/wp/v2/widgets`
- [x] `/wp/v2/widget-types`
- [ ] `/wp/v2/sidebars`

## System Endpoints

- [x] `/wp/v2/settings`
- [x] `/wp/v2/themes`
- [x] `/wp/v2/plugins`
- [x] `/wp/v2/users`
- [x] `/wp/v2/users/<user_id>/application-passwords`

## Global Styles

- [ ] `/wp/v2/global-styles/<id>`
- [ ] `/wp/v2/global-styles/<id>/revisions`

## Additional Implemented Endpoints

- [x] `/wp-block-editor/v1/settings`
- [x] `/wp-site-health/v1/tests`
- [x] `/wp/v2` (API root)
