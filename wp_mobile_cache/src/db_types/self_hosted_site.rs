use crate::RowId;

/// Represents a self-hosted WordPress site (domain model).
///
/// This type contains the site data without database metadata.
/// Use this when creating or updating sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfHostedSite {
    pub url: String,
    pub api_root: String,
}

/// Represents a self-hosted WordPress site in the database.
///
/// This type includes the database rowid along with the site data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbSelfHostedSite {
    pub row_id: RowId,
    pub url: String,
    pub api_root: String,
}
