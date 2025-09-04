use super::{AsNamespace, DerivedRequest, WpNamespace};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SiteSettingsRequest {
    #[contextual_get(url = "/settings", output = crate::site_settings::SparseSiteSettings, filter_by = crate::site_settings::SparseSiteSettingsField)]
    Retrieve,
    #[post(url = "/settings", params = &crate::site_settings::SiteSettingsUpdateParams, output = crate::site_settings::SiteSettingsWithEditContext)]
    Update,
}

impl DerivedRequest for SiteSettingsRequest {
    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
        site_settings::SparseSiteSettingsFieldWithEmbedContext,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn retrieve_with_edit_context(endpoint: SiteSettingsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(),
            "/settings?context=edit",
        );
    }

    #[rstest]
    #[case(&[SparseSiteSettingsFieldWithEmbedContext::Title], "/settings?context=embed&_fields=title")]
    #[case(&[SparseSiteSettingsFieldWithEmbedContext::Email, SparseSiteSettingsFieldWithEmbedContext::Timezone], "/settings?context=embed&_fields=email%2Ctimezone")]
    fn filter_retrieve_with_embed_context(
        endpoint: SiteSettingsRequestEndpoint,
        #[case] fields: &[SparseSiteSettingsFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_embed_context(fields),
            expected_path,
        );
    }

    #[rstest]
    fn update(endpoint: SiteSettingsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.update(), "/settings");
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> SiteSettingsRequestEndpoint {
        SiteSettingsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
