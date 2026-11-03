use crate::wp_com::language::WpComRemoteLanguageMap;
use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::WpComNamespace,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum LanguagesRequest {
    #[get(url = "/i18n/language-names", output = WpComRemoteLanguageMap)]
    Get,
}

impl DerivedRequest for LanguagesRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::V2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::{
            endpoint::tests::{
                fixture_wp_com_api_url_resolver, language_provider, validate_wp_com_v2_endpoint,
            },
            language::WPComLanguage,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn v2_namespace_uses_underscore_locale(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) {
        let endpoint = LanguagesRequestEndpoint::with_language_provider(
            fixture_wp_com_api_url_resolver,
            language_provider(Some(WPComLanguage::Spanish)),
        );
        validate_wp_com_v2_endpoint(endpoint.get(), "/i18n/language-names?_locale=es");
    }
}
