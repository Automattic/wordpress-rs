/// Strategy for list sync operations.
///
/// Controls how much work is done when syncing a list:
/// - `MetadataOnly`: Fetch list structure (IDs, modified_gmt) but don't fetch entity data
/// - `Full`: Fetch metadata AND fetch missing/stale entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, uniffi::Enum)]
pub enum SyncStrategy {
    /// Only sync list metadata (IDs, ordering, pagination).
    /// Entity data is not fetched - useful when only list structure is needed.
    MetadataOnly,

    /// Full sync: fetch metadata, then fetch any missing or stale entities.
    /// This is the typical behavior for displaying a list.
    #[default]
    Full,
}
