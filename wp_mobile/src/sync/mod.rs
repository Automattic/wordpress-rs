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
//! - [`DbEntityState`] - Fetch state (Missing, Fetching, Cached, Stale, Failed)
//! - [`CollectionItem`] - Combines metadata with state
//! - [`MetadataFetchResult`] - Result of metadata-only fetch
//! - [`SyncResult`] - Result of sync operation
//! - [`SyncStrategy`] - Strategy for list sync (MetadataOnly vs Full)
//!
//! ## Service Types
//!
//! - [`EntityStateService`] - Stateless service for entity state operations
//! - [`EntityStateReader`] - Read-only access to entity states (trait)
//! - [`ListMetadataReader`] - Read-only access to list metadata (trait)
//!
//! ## Collection Types
//!
//! - `MetadataCollectionCore` - Core collection infrastructure for query logic (in `collection` module)
//!
//! Entity-specific collections (e.g., `PostMetadataCollectionWithEditContext`) compose
//! `MetadataCollectionCore` and add their own sync logic.
//!
//! See `wp_mobile/docs/design/metadata_collection_v3.md` for full design details.

mod collection_item;
mod entity_metadata;
mod entity_state_store;
mod list_metadata_reader;
mod metadata_fetch_result;
mod sync_result;
mod sync_strategy;

pub use collection_item::CollectionItem;
pub use entity_metadata::EntityMetadata;
pub use entity_state_store::{EntityStateReader, EntityStateReaderImpl, EntityStateService};
pub use list_metadata_reader::{ListInfo, ListMetadataReader};
pub use metadata_fetch_result::MetadataFetchResult;
pub use sync_result::SyncResult;
pub use sync_strategy::SyncStrategy;
pub use wp_mobile_cache::repository::entity_state::DbEntityState;
