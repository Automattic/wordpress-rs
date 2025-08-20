use crate::SparseField;
use crate::themes::{
    SparseThemeFieldWithEditContext, SparseThemeFieldWithEmbedContext,
    SparseThemeFieldWithViewContext, ThemeStylesheet,
};
use wp_derive_request_builder::WpDerivedRequest;

use super::{AsNamespace, DerivedRequest, WpNamespace};
#[derive(WpDerivedRequest)]
enum ThemesRequest {
    #[contextual_get(url = "/themes", params = &crate::themes::ThemeListParams, output = Vec<crate::themes::SparseTheme>, filter_by = crate::themes::SparseThemeField)]
    List,
    #[contextual_get(url = "/themes/<theme_stylesheet>", output = crate::themes::SparseTheme, filter_by = crate::themes::SparseThemeField)]
    Retrieve,
}

impl DerivedRequest for ThemesRequest {
    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

super::macros::default_sparse_field_implementation_from_field_name!(
    SparseThemeFieldWithEditContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparseThemeFieldWithEmbedContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparseThemeFieldWithViewContext
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        generate,
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
        themes::{ThemeListParams, ThemeStatus},
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(ThemeListParams::default(), "")]
    #[case(generate!(ThemeListParams, (status, Some(ThemeStatus::Active))), "status=active")]
    #[case(generate!(ThemeListParams, (status, Some(ThemeStatus::Inactive))), "status=inactive")]
    fn list_themes(
        endpoint: ThemesRequestEndpoint,
        #[case] params: ThemeListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/themes?context={context}")
            } else {
                format!("/themes?context={context}&{expected_additional_params}")
            }
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&params),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(ThemeListParams::default(), &[], "/themes?context=edit&_fields=")]
    #[case(ThemeListParams::default(), ALL_SPARSE_THEME_FIELDS_WITH_EDIT_CONTEXT, &format!("/themes?context=edit&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_THEME_FIELDS_WITH_EDIT_CONTEXT}"))]
    #[case(generate!(ThemeListParams, (status, Some(ThemeStatus::Active))), &[], "/themes?context=edit&status=active&_fields=")]
    #[case(generate!(ThemeListParams, (status, Some(ThemeStatus::Inactive))), ALL_SPARSE_THEME_FIELDS_WITH_EDIT_CONTEXT, &format!("/themes?context=edit&status=inactive&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_THEME_FIELDS_WITH_EDIT_CONTEXT}"))]
    fn filter_list_themes_with_edit_context(
        endpoint: ThemesRequestEndpoint,
        #[case] params: ThemeListParams,
        #[case] fields: &[SparseThemeFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(ThemeListParams::default(), &[], "/themes?context=embed&_fields=")]
    #[case(ThemeListParams::default(), ALL_SPARSE_THEME_FIELDS_WITH_EMBED_CONTEXT, &format!("/themes?context=embed&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_THEME_FIELDS_WITH_EMBED_CONTEXT}"))]
    #[case(generate!(ThemeListParams, (status, Some(ThemeStatus::Active))), &[], "/themes?context=embed&status=active&_fields=")]
    #[case(generate!(ThemeListParams, (status, Some(ThemeStatus::Inactive))), ALL_SPARSE_THEME_FIELDS_WITH_EMBED_CONTEXT, &format!("/themes?context=embed&status=inactive&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_THEME_FIELDS_WITH_EMBED_CONTEXT}"))]
    fn filter_list_themes_with_embed_context(
        endpoint: ThemesRequestEndpoint,
        #[case] params: ThemeListParams,
        #[case] fields: &[SparseThemeFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_embed_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(ThemeListParams::default(), &[], "/themes?context=view&_fields=")]
    #[case(ThemeListParams::default(), ALL_SPARSE_THEME_FIELDS_WITH_VIEW_CONTEXT, &format!("/themes?context=view&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_THEME_FIELDS_WITH_VIEW_CONTEXT}"))]
    #[case(generate!(ThemeListParams, (status, Some(ThemeStatus::Active))), &[], "/themes?context=view&status=active&_fields=")]
    #[case(generate!(ThemeListParams, (status, Some(ThemeStatus::Inactive))), ALL_SPARSE_THEME_FIELDS_WITH_VIEW_CONTEXT, &format!("/themes?context=view&status=inactive&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_THEME_FIELDS_WITH_VIEW_CONTEXT}"))]
    fn filter_list_themes_with_view_context(
        endpoint: ThemesRequestEndpoint,
        #[case] params: ThemeListParams,
        #[case] fields: &[SparseThemeFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    fn retrieve_theme(endpoint: ThemesRequestEndpoint) {
        let theme_stylesheet: ThemeStylesheet = "foo".into();
        let expected_path = |context: &str| format!("/themes/foo?context={context}");
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&theme_stylesheet),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&theme_stylesheet),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&theme_stylesheet),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(&[], "/themes/foo?context=edit&_fields=")]
    #[case(&[SparseThemeFieldWithEditContext::Author], "/themes/foo?context=edit&_fields=author")]
    #[case(ALL_SPARSE_THEME_FIELDS_WITH_EDIT_CONTEXT, &format!("/themes/foo?context=edit&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_THEME_FIELDS_WITH_EDIT_CONTEXT}"))]
    fn filter_retrieve_theme_with_edit_context(
        endpoint: ThemesRequestEndpoint,
        #[case] fields: &[SparseThemeFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_edit_context(&"foo".into(), fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(&[], "/themes/foo?context=embed&_fields=")]
    #[case(&[SparseThemeFieldWithEmbedContext::Author], "/themes/foo?context=embed&_fields=author")]
    #[case(ALL_SPARSE_THEME_FIELDS_WITH_EMBED_CONTEXT, &format!("/themes/foo?context=embed&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_THEME_FIELDS_WITH_EMBED_CONTEXT}"))]
    fn filter_retrieve_theme_with_embed_context(
        endpoint: ThemesRequestEndpoint,
        #[case] fields: &[SparseThemeFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_embed_context(&"foo".into(), fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(&[], "/themes/foo?context=view&_fields=")]
    #[case(&[SparseThemeFieldWithViewContext::Author], "/themes/foo?context=view&_fields=author")]
    #[case(ALL_SPARSE_THEME_FIELDS_WITH_VIEW_CONTEXT, &format!("/themes/foo?context=view&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_THEME_FIELDS_WITH_VIEW_CONTEXT}"))]
    fn filter_retrieve_theme_with_view_context(
        endpoint: ThemesRequestEndpoint,
        #[case] fields: &[SparseThemeFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(&"foo".into(), fields),
            expected_path,
        );
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_THEME_FIELDS_WITH_EDIT_CONTEXT: &str = "_fields=stylesheet%2Ctemplate%2Crequires_php%2Crequires_wp%2Ctextdomain%2Cversion%2Cscreenshot%2Cauthor%2Cauthor_uri%2Cdescription%2Cname%2Ctags%2Ctheme_uri%2Cstatus%2Cis_block_theme%2Cstylesheet_uri%2Ctemplate_uri%2Ctheme_supports";
    const ALL_SPARSE_THEME_FIELDS_WITH_EDIT_CONTEXT: &[SparseThemeFieldWithEditContext; 18] = &[
        SparseThemeFieldWithEditContext::Stylesheet,
        SparseThemeFieldWithEditContext::Template,
        SparseThemeFieldWithEditContext::RequiresPhp,
        SparseThemeFieldWithEditContext::RequiresWp,
        SparseThemeFieldWithEditContext::Textdomain,
        SparseThemeFieldWithEditContext::Version,
        SparseThemeFieldWithEditContext::Screenshot,
        SparseThemeFieldWithEditContext::Author,
        SparseThemeFieldWithEditContext::AuthorUri,
        SparseThemeFieldWithEditContext::Description,
        SparseThemeFieldWithEditContext::Name,
        SparseThemeFieldWithEditContext::Tags,
        SparseThemeFieldWithEditContext::ThemeUri,
        SparseThemeFieldWithEditContext::Status,
        SparseThemeFieldWithEditContext::IsBlockTheme,
        SparseThemeFieldWithEditContext::StylesheetUri,
        SparseThemeFieldWithEditContext::TemplateUri,
        SparseThemeFieldWithEditContext::ThemeSupports,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_THEME_FIELDS_WITH_EMBED_CONTEXT: &str = "_fields=stylesheet%2Ctemplate%2Crequires_php%2Crequires_wp%2Ctextdomain%2Cversion%2Cscreenshot%2Cauthor%2Cauthor_uri%2Cdescription%2Cname%2Ctags%2Ctheme_uri%2Cstatus%2Cis_block_theme%2Cstylesheet_uri%2Ctemplate_uri%2Ctheme_supports";
    const ALL_SPARSE_THEME_FIELDS_WITH_EMBED_CONTEXT: &[SparseThemeFieldWithEmbedContext; 18] = &[
        SparseThemeFieldWithEmbedContext::Stylesheet,
        SparseThemeFieldWithEmbedContext::Template,
        SparseThemeFieldWithEmbedContext::RequiresPhp,
        SparseThemeFieldWithEmbedContext::RequiresWp,
        SparseThemeFieldWithEmbedContext::Textdomain,
        SparseThemeFieldWithEmbedContext::Version,
        SparseThemeFieldWithEmbedContext::Screenshot,
        SparseThemeFieldWithEmbedContext::Author,
        SparseThemeFieldWithEmbedContext::AuthorUri,
        SparseThemeFieldWithEmbedContext::Description,
        SparseThemeFieldWithEmbedContext::Name,
        SparseThemeFieldWithEmbedContext::Tags,
        SparseThemeFieldWithEmbedContext::ThemeUri,
        SparseThemeFieldWithEmbedContext::Status,
        SparseThemeFieldWithEmbedContext::IsBlockTheme,
        SparseThemeFieldWithEmbedContext::StylesheetUri,
        SparseThemeFieldWithEmbedContext::TemplateUri,
        SparseThemeFieldWithEmbedContext::ThemeSupports,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_THEME_FIELDS_WITH_VIEW_CONTEXT: &str = "_fields=stylesheet%2Ctemplate%2Crequires_php%2Crequires_wp%2Ctextdomain%2Cversion%2Cscreenshot%2Cauthor%2Cauthor_uri%2Cdescription%2Cname%2Ctags%2Ctheme_uri%2Cstatus%2Cis_block_theme%2Cstylesheet_uri%2Ctemplate_uri%2Ctheme_supports";
    const ALL_SPARSE_THEME_FIELDS_WITH_VIEW_CONTEXT: &[SparseThemeFieldWithViewContext; 18] = &[
        SparseThemeFieldWithViewContext::Stylesheet,
        SparseThemeFieldWithViewContext::Template,
        SparseThemeFieldWithViewContext::RequiresPhp,
        SparseThemeFieldWithViewContext::RequiresWp,
        SparseThemeFieldWithViewContext::Textdomain,
        SparseThemeFieldWithViewContext::Version,
        SparseThemeFieldWithViewContext::Screenshot,
        SparseThemeFieldWithViewContext::Author,
        SparseThemeFieldWithViewContext::AuthorUri,
        SparseThemeFieldWithViewContext::Description,
        SparseThemeFieldWithViewContext::Name,
        SparseThemeFieldWithViewContext::Tags,
        SparseThemeFieldWithViewContext::ThemeUri,
        SparseThemeFieldWithViewContext::Status,
        SparseThemeFieldWithViewContext::IsBlockTheme,
        SparseThemeFieldWithViewContext::StylesheetUri,
        SparseThemeFieldWithViewContext::TemplateUri,
        SparseThemeFieldWithViewContext::ThemeSupports,
    ];

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ThemesRequestEndpoint {
        ThemesRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
