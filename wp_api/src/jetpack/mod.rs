use crate::request::endpoint::AsNamespace;

pub mod client;
pub mod connection;
pub mod endpoint;

pub(crate) struct JetpackNamespace();

impl AsNamespace for JetpackNamespace {
    fn namespace_value(&self) -> &'static str {
        "/jetpack/v4"
    }
}
