mod collection_error;
mod core;
mod fetch_error;
mod fetch_result;
pub(crate) mod post_collection;
pub(crate) mod post_metadata_collection;
pub(crate) mod post_type_collection;
mod stateless_collection;

pub use collection_error::CollectionError;
pub use core::MetadataCollectionCore;
pub use fetch_error::FetchError;
pub use fetch_result::FetchResult;
pub use post_metadata_collection::{
    PostItemState, PostMetadataCollectionItem, PostMetadataCollectionWithEditContext,
};
pub use post_type_collection::PostTypeCollectionWithEditContext;
pub use stateless_collection::StatelessCollection;

/// Macro to create UniFFI-compatible item state enums for metadata collections.
///
/// This macro generates a type-safe enum that combines sync status with data availability.
/// Data presence is encoded in the variant itself, eliminating inconsistent states.
///
/// # Parameters
/// - `$state_name`: Name for the enum (e.g., `PostItemState`)
/// - `$full_entity_type`: The FullEntity wrapper type (e.g., `FullEntityAnyPostWithEditContext`)
///
/// # Generated Variants
/// - `Missing`: No cached data, needs fetch
/// - `Fetching`: Fetch in progress, no cached data
/// - `FetchingWithData { data }`: Fetch in progress, showing cached data
/// - `Fresh { data }`: Fresh data
/// - `Stale { data }`: Outdated cached data
/// - `Failed { error }`: Fetch failed, no cached data
/// - `FailedWithData { error, data }`: Fetch failed, showing cached data
///
/// # Usage
/// ```ignore
/// wp_mobile_item_state!(PostItemState, FullEntityAnyPostWithEditContext);
/// ```
#[macro_export]
macro_rules! wp_mobile_item_state {
    ($state_name:ident, $full_entity_type:ty) => {
        /// Combined state and data for an item in a metadata collection.
        ///
        /// This enum provides type-safe representation of item state with associated data.
        /// Data presence is encoded in the variant itself, eliminating the need for
        /// separate `state` and `data` fields.
        #[derive(uniffi::Enum)]
        pub enum $state_name {
            /// No cached data available, needs fetch
            Missing,

            /// Fetch in progress, no cached data to show
            Fetching,

            /// Fetch in progress, showing cached data while loading
            FetchingWithData { data: $full_entity_type },

            /// Fresh data, no fetch needed
            Fresh { data: $full_entity_type },

            /// Cached data is outdated, could benefit from refresh
            Stale { data: $full_entity_type },

            /// Fetch failed, no cached data available
            Failed { error: String },

            /// Fetch failed, showing last known cached data
            FailedWithData {
                error: String,
                data: $full_entity_type,
            },
        }
    };
}

/// Macro to create UniFFI-compatible metadata collection item types.
///
/// This macro generates both the state enum and the collection item struct for
/// metadata-driven collections. The generated types are suitable for use across
/// language boundaries via UniFFI.
///
/// # Parameters
/// - `$item_name`: Name for the collection item struct (e.g., `PostMetadataCollectionItem`)
/// - `$state_name`: Name for the state enum (e.g., `PostItemState`)
/// - `$full_entity_type`: The FullEntity wrapper type (e.g., `FullEntityAnyPostWithEditContext`)
///
/// # Generated Types
///
/// ## State Enum (`$state_name`)
/// - `Missing`: No cached data, needs fetch
/// - `Fetching`: Fetch in progress, no cached data
/// - `FetchingWithData { data }`: Fetch in progress, showing cached data
/// - `Fresh { data }`: Fresh data
/// - `Stale { data }`: Outdated cached data
/// - `Failed { error }`: Fetch failed, no cached data
/// - `FailedWithData { error, data }`: Fetch failed, showing cached data
///
/// ## Collection Item Struct (`$item_name`)
/// - `id: i64`: The entity ID
/// - `parent: Option<i64>`: Parent entity ID (from list metadata, for hierarchical types)
/// - `menu_order: Option<i64>`: Menu order (from list metadata, for hierarchical types)
/// - `state: $state_name`: The combined state and data
///
/// # Usage
/// ```ignore
/// wp_mobile_metadata_item!(
///     PostMetadataCollectionItem,
///     PostItemState,
///     FullEntityAnyPostWithEditContext
/// );
/// ```
#[macro_export]
macro_rules! wp_mobile_metadata_item {
    ($item_name:ident, $state_name:ident, $full_entity_type:ty) => {
        // Generate the state enum using the existing macro
        $crate::wp_mobile_item_state!($state_name, $full_entity_type);

        /// Item in a metadata collection with type-safe state representation.
        ///
        /// The `state` enum encodes both the sync status and data availability,
        /// making it impossible to have inconsistent combinations.
        ///
        /// The `parent` and `menu_order` fields come from the list metadata store,
        /// making them available immediately without waiting for full entity data
        /// to be fetched. This enables building hierarchical views (like page trees)
        /// as soon as the list structure is known.
        #[derive(uniffi::Record)]
        pub struct $item_name {
            /// The entity ID
            pub id: i64,

            /// Parent entity ID (from list metadata, for hierarchical post types like pages)
            ///
            /// This value comes from the list metadata, so it's available immediately
            /// without waiting for the full post data to be fetched.
            pub parent: Option<i64>,

            /// Menu order (from list metadata, for hierarchical post types)
            ///
            /// This value comes from the list metadata, so it's available immediately
            /// without waiting for the full post data to be fetched.
            pub menu_order: Option<i64>,

            /// Combined state and data - see the state enum for variants
            pub state: $state_name,
        }

        // Generate From trait: DbEntityState + data -> ItemState
        impl From<($crate::sync::DbEntityState, Option<$full_entity_type>)> for $state_name {
            /// Convert DbEntityState + optional cached data into ItemState.
            ///
            /// This encodes the business logic for how fetch state and data availability
            /// combine into user-facing states:
            ///
            /// - `Missing + no data` → Show placeholder (need to fetch)
            /// - `Missing + has data` → Show stale data (app restart scenario)
            /// - `Fetching + no data` → Show loading spinner
            /// - `Fetching + has data` → Show data with loading indicator
            /// - `Fresh + has data` → Show fresh data
            /// - `Fresh + no data` → Defensive fallback to Missing
            /// - `Stale + has data` → Show outdated data
            /// - `Stale + no data` → Defensive fallback to Missing
            /// - `Failed + no data` → Show error message
            /// - `Failed + has data` → Show data with error indicator
            fn from(
                (state, data): ($crate::sync::DbEntityState, Option<$full_entity_type>),
            ) -> Self {
                match (state, data) {
                    // Missing state
                    ($crate::sync::DbEntityState::Missing, None) => $state_name::Missing,
                    ($crate::sync::DbEntityState::Missing, Some(data)) => {
                        $state_name::Stale { data }
                    }

                    // Fetching state
                    ($crate::sync::DbEntityState::Fetching, None) => $state_name::Fetching,
                    ($crate::sync::DbEntityState::Fetching, Some(data)) => {
                        $state_name::FetchingWithData { data }
                    }

                    // Fresh state (should always have data, but handle gracefully)
                    ($crate::sync::DbEntityState::Fresh, Some(data)) => $state_name::Fresh { data },
                    ($crate::sync::DbEntityState::Fresh, None) => $state_name::Missing,

                    // Stale state (should always have data, but handle gracefully)
                    ($crate::sync::DbEntityState::Stale, Some(data)) => $state_name::Stale { data },
                    ($crate::sync::DbEntityState::Stale, None) => $state_name::Missing,

                    // Failed state
                    ($crate::sync::DbEntityState::Failed { error }, None) => {
                        $state_name::Failed { error }
                    }
                    ($crate::sync::DbEntityState::Failed { error }, Some(data)) => {
                        $state_name::FailedWithData { error, data }
                    }
                }
            }
        }

        // Generate From trait: CollectionItem + data -> MetadataCollectionItem
        impl From<($crate::sync::CollectionItem, Option<$full_entity_type>)> for $item_name {
            /// Convert CollectionItem + optional cached data into MetadataCollectionItem.
            ///
            /// Extracts metadata fields (id, parent, menu_order) and converts the state+data
            /// into a type-safe ItemState.
            fn from(
                (item, data): ($crate::sync::CollectionItem, Option<$full_entity_type>),
            ) -> Self {
                $item_name {
                    id: item.id(),
                    parent: item.metadata.parent,
                    menu_order: item.metadata.menu_order,
                    state: $state_name::from((item.state, data)),
                }
            }
        }
    };
}

/// Macro to create UniFFI-compatible post collection wrappers
///
/// This macro generates a wrapper type for `PostCollection<T>` that can be used
/// across language boundaries via UniFFI. The generated type includes methods for
/// fetching from network and loading from cache.
///
/// # Parameters
/// - `$wrapper_name`: Name for the wrapper struct (e.g., `PostCollectionWithEditContext`)
/// - `$entity_name`: Entity name for FullEntity type (e.g., `AnyPostWithEditContext`)
/// - `$data_type`: The underlying data type (e.g., `wp_api::posts::AnyPostWithEditContext`)
///
/// # Usage
/// ```ignore
/// wp_mobile_post_collection!(
///     PostCollectionWithEditContext,
///     AnyPostWithEditContext,
///     wp_api::posts::AnyPostWithEditContext
/// );
/// ```
#[macro_export]
macro_rules! wp_mobile_post_collection {
    ($wrapper_name:ident, $entity_name:ident, $data_type:ty) => {
        paste::paste! {
            #[derive(uniffi::Object)]
            pub struct $wrapper_name(pub $crate::collection::post_collection::PostCollection<$data_type>);

            impl From<$crate::collection::post_collection::PostCollection<$data_type>> for $wrapper_name {
                fn from(value: $crate::collection::post_collection::PostCollection<$data_type>) -> Self {
                    Self(value)
                }
            }

            #[uniffi::export]
            impl $wrapper_name {
                /// Fetch a specific page from the network
                ///
                /// This calls the network API and upserts results to the database.
                /// After successful fetch, the database change will trigger observers
                /// who can then call load_data() to get updated results.
                ///
                /// # Arguments
                /// * `page` - Page number to fetch (1-indexed)
                /// * `per_page` - Number of posts per page
                ///
                /// # Returns
                /// - `Ok(FetchResult)` with entity IDs and pagination info
                /// - `Err(FetchError)` if network or database error occurs
                ///
                /// # Note
                /// This is a stateless operation - the collection doesn't track
                /// which pages have been fetched. ViewModels manage pagination state.
                pub async fn fetch_page(
                    &self,
                    page: u32,
                    per_page: u32,
                ) -> Result<$crate::collection::FetchResult, $crate::collection::FetchError> {
                    self.0.fetch_page(page, per_page).await
                }

                /// Load all cached items matching this collection's filter
                ///
                /// This queries the database and returns all posts that match
                /// the collection's filter criteria. It's an expensive operation
                /// that re-queries on every call (stateless behavior).
                ///
                /// Returns:
                /// - `Ok(Vec<FullEntity>>)` with all matching posts from cache
                /// - `Err(CollectionError)` if database error occurs
                ///
                /// # Note
                /// This async function is exported to client platforms (Kotlin/Swift) where it
                /// will be executed on a background thread. The underlying Rust implementation
                /// is synchronous as rusqlite doesn't support async operations.
                pub async fn load_data(
                    &self,
                ) -> Result<Vec<[<FullEntity $entity_name>]>, $crate::collection::CollectionError> {
                    self.0
                        .load_data()
                        .map(|full_entities| {
                            full_entities
                                .into_iter()
                                .map(|full_entity| full_entity.into())
                                .collect()
                        })
                }

                /// Check if a database update is relevant to this collection
                ///
                /// Returns true if the update might affect posts in this collection.
                /// Used by platform-specific observable wrappers to determine
                /// whether to notify observers.
                pub fn is_relevant_update(&self, hook: &wp_mobile_cache::UpdateHook) -> bool {
                    self.0.is_relevant_update(hook)
                }

                /// Get the filter for this collection
                pub fn filter(&self) -> $crate::filters::AnyPostFilter {
                    self.0.filter().clone()
                }
            }
        }
    };
}

/// Macro to create UniFFI-compatible stateless collection wrappers
///
/// This macro generates a wrapper type for `StatelessCollection<T>` that can be used
/// across language boundaries via UniFFI. The generated type includes methods for
/// loading data and checking update relevance.
///
/// The macro automatically generates the collection name by prepending "All" and
/// appending "Collection" to the entity name, and also auto-generates the full
/// entity type name by prepending "FullEntity" to the entity name.
///
/// # Parameters
/// - `$entity_name`: Base name for the entity (e.g., `AnyPostWithEditContext`)
/// - `$data_type`: The underlying data type (e.g., `wp_api::posts::AnyPostWithEditContext`)
///
/// # Usage
/// ```ignore
/// wp_mobile_stateless_collection!(
///     AnyPostWithEditContext,
///     wp_api::posts::AnyPostWithEditContext
/// );
/// ```
///
/// This generates:
/// - `AllAnyPostWithEditContextCollection` - the collection wrapper type
/// - Uses `FullEntityAnyPostWithEditContext` - for the return type
#[macro_export]
macro_rules! wp_mobile_stateless_collection {
    ($entity_name:ident, $data_type:ty) => {
        paste::paste! {
            #[derive(uniffi::Object)]
            pub struct [<All $entity_name Collection>](
                pub $crate::collection::StatelessCollection<
                    wp_mobile_cache::entity::FullEntity<$data_type>,
                >,
            );

            impl From<
                    $crate::collection::StatelessCollection<
                        wp_mobile_cache::entity::FullEntity<$data_type>,
                    >,
                > for [<All $entity_name Collection>]
            {
                fn from(
                    value: $crate::collection::StatelessCollection<
                        wp_mobile_cache::entity::FullEntity<$data_type>,
                    >,
                ) -> Self {
                    Self(value)
                }
            }

            #[uniffi::export]
            impl [<All $entity_name Collection>] {
                /// Load all items in the collection from the database
                ///
                /// This is an expensive operation that reads from the database each time.
                /// It returns all items currently stored in the database that match the
                /// collection's criteria (site, context, etc.).
                ///
                /// Returns:
                /// - Ok(Vec<FullEntity>) - All items in the collection (may be empty)
                /// - Err(CollectionError) if a database error occurred
                ///
                /// # Note
                /// This async function is exported to client platforms (Kotlin/Swift) where it
                /// will be executed on a background thread. The underlying Rust implementation
                /// is synchronous as rusqlite doesn't support async operations.
                pub async fn load_data(
                    &self,
                ) -> Result<Vec<[<FullEntity $entity_name>]>, $crate::collection::CollectionError> {
                    self.0
                        .load_data()
                        .map(|full_entities| {
                            full_entities
                                .into_iter()
                                .map(|full_entity| full_entity.into())
                                .collect()
                        })
                        .map_err(|e| e.into())
                }

                /// Check if a database update is relevant to this collection
                ///
                /// Returns true if the updated table is one of the tables this collection monitors.
                /// This allows platform-specific observable wrappers to determine whether they should
                /// notify observers about a database change.
                pub fn is_relevant_update(&self, hook: &wp_mobile_cache::UpdateHook) -> bool {
                    self.0.is_relevant_update(hook)
                }
            }
        }
    };
}
