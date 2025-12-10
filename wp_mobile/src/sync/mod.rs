//! Metadata-based sync infrastructure for efficient list fetching.
//!
//! This module provides types for a "smart sync" strategy:
//! 1. Fetch lightweight metadata (id + modified_gmt) to define list structure
//! 2. Show cached entities immediately, with loading placeholders for missing items
//! 3. Selectively fetch only entities that are missing or stale
//!
//! ## Key Types
//!
//! - [`EntityMetadata`] - Lightweight metadata (id + optional modified_gmt)
//! - [`EntityState`] - Fetch state (Missing, Fetching, Cached, Stale, Failed)
//! - [`CollectionItem`] - Combines metadata with state
//! - [`MetadataFetchResult`] - Result of metadata-only fetch
//! - [`SyncResult`] - Result of sync operation
//!
//! ## Store Types
//!
//! - [`EntityStateStore`] - Tracks fetch state per entity (read-write)
//! - [`EntityStateReader`] - Read-only access to entity states (trait)
//! - [`ListMetadataStore`] - Tracks list structure per filter (read-write)
//! - [`ListMetadataReader`] - Read-only access to list metadata (trait)
//!
//! See `wp_mobile/docs/design/metadata_collection_v3.md` for full design details.

mod collection_item;
mod entity_metadata;
mod entity_state;
mod entity_state_store;
mod list_metadata_store;
mod metadata_fetch_result;
mod sync_result;

pub use collection_item::CollectionItem;
pub use entity_metadata::EntityMetadata;
pub use entity_state::EntityState;
pub use entity_state_store::{EntityStateReader, EntityStateStore};
pub use list_metadata_store::{ListMetadataReader, ListMetadataStore};
pub use metadata_fetch_result::MetadataFetchResult;
pub use sync_result::SyncResult;
