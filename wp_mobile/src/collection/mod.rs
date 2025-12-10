mod collection_error;
mod fetch_error;
mod fetch_result;
pub(crate) mod post_collection;
pub(crate) mod post_metadata_collection;
mod stateless_collection;

pub use collection_error::CollectionError;
pub use fetch_error::FetchError;
pub use fetch_result::FetchResult;
pub use post_metadata_collection::{PostMetadataCollectionItem, PostMetadataCollectionWithEditContext};
pub use stateless_collection::StatelessCollection;

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
