use wp_api::request::endpoint::AsNamespace;

pub mod connection_endpoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum JetpackNamespace {
    JetpackV4,
}

impl AsNamespace for JetpackNamespace {
    fn as_str(&self) -> &str {
        match self {
            Self::JetpackV4 => "/jetpack/v4",
        }
    }
}
