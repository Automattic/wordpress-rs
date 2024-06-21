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
