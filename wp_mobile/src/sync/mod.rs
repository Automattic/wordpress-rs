//! Metadata-based sync infrastructure for efficient list fetching.
//!
//! This module provides types and traits for a "smart sync" strategy:
//! 1. Fetch lightweight metadata (id + modified_gmt) to define list structure
//! 2. Show cached entities immediately, with loading placeholders for missing items
//! 3. Selectively fetch only entities that are missing or stale
//!
//! See `wp_mobile/docs/design/metadata_collection_design.md` for full design details.

mod entity_metadata;
mod list_item;
mod syncable_entity;

pub use entity_metadata::EntityMetadata;
pub use list_item::{HasId, ListItem};
pub use syncable_entity::SyncableEntity;
