use std::fmt::Display;

use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseApplicationPassword {
    #[WpContext(edit, embed, view)]
    pub uuid: Option<ApplicationPasswordUuid>,
    #[WpContext(edit, embed, view)]
    pub app_id: Option<ApplicationPasswordAppId>,
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit, view)]
    pub created: Option<String>,
    #[WpContextualOption]
    #[WpContext(edit, view)]
    pub last_used: Option<String>,
    #[WpContextualOption]
    #[WpContext(edit, view)]
    pub last_ip: Option<IpAddress>,
    #[WpContextualOption]
    #[WpContext(edit)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SparseApplicationPasswordField {
    Uuid,
    AppId,
    Name,
    Created,
    LastUsed,
    LastIp,
    Password,
}

impl crate::SparseField for SparseApplicationPasswordField {
    fn as_str(&self) -> &str {
        match self {
            Self::Uuid => "uuid",
            Self::AppId => "app_id",
            Self::Name => "name",
            Self::Created => "created",
            Self::LastUsed => "last_used",
            Self::LastIp => "last_ip",
            Self::Password => "password",
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record)]
#[serde(transparent)]
pub struct ApplicationPasswordUuid {
    pub uuid: String,
}

impl Display for ApplicationPasswordUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.uuid)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record)]
#[serde(transparent)]
pub struct ApplicationPasswordAppId {
    pub app_id: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record)]
#[serde(transparent)]
pub struct IpAddress {
    #[serde(alias = "last_ip")]
    pub value: String,
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct ApplicationPasswordCreateParams {
    /// A UUID provided by the application to uniquely identify it.
    /// It is recommended to use an UUID v5 with the URL or DNS namespace.
    pub app_id: Option<String>,
    /// The name of the application password.
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ApplicationPasswordDeleteResponse {
    pub deleted: bool,
    pub previous: ApplicationPasswordWithEditContext,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ApplicationPasswordDeleteAllResponse {
    pub deleted: bool,
    pub count: i32,
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct ApplicationPasswordUpdateParams {
    /// A UUID provided by the application to uniquely identify it.
    /// It is recommended to use an UUID v5 with the URL or DNS namespace.
    pub app_id: Option<String>,
    /// The name of the application password.
    pub name: String,
}
