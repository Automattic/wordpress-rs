use crate::wp_com::language::{LanguagesGetParams, WpComRemoteLanguageMap};
use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::WpComNamespace,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum LanguagesRequest {
    #[get(url = "/i18n/language-names", params = &LanguagesGetParams, output = WpComRemoteLanguageMap)]
    Get,
}

impl DerivedRequest for LanguagesRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::V2
    }
}
