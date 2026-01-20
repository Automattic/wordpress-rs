use wp_api::themes::{
    SparseThemeFieldWithEditContext, SparseThemeFieldWithEmbedContext,
    SparseThemeFieldWithViewContext, ThemeListParams, ThemeStatus, ThemeStylesheet, ThemeSupports,
    ThemeSupportsData,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: ThemeListParams) {
    api_client()
        .themes()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: ThemeListParams) {
    api_client()
        .themes()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: ThemeListParams) {
    api_client()
        .themes()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
async fn retrieve_with_edit_context(#[case] stylesheet: ThemeStylesheet) {
    api_client()
        .themes()
        .retrieve_with_edit_context(&stylesheet)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
async fn retrieve_with_embed_context(#[case] stylesheet: ThemeStylesheet) {
    api_client()
        .themes()
        .retrieve_with_embed_context(&stylesheet)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
async fn retrieve_with_view_context(#[case] stylesheet: ThemeStylesheet) {
    api_client()
        .themes()
        .retrieve_with_view_context(&stylesheet)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_theme_supports_is_a_bool() {
    let theme = api_client()
        .themes()
        .retrieve_with_view_context(&THEME_TWENTY_TWENTY_FOUR.into())
        .await
        .assert_response()
        .data;
    assert!(matches!(
        theme
            .theme_supports
            .expect("'twentytwentyfour' theme includes the 'theme_supports' field")
            .get(&ThemeSupports::BlockTemplates)
            .expect(
                "'twentytwentyfour' theme's 'theme_supports' field includes 'block-templates' type"
            ),
        ThemeSupportsData::Bool(true)
    ));
}

#[tokio::test]
#[parallel]
async fn retrieve_theme_supports_is_a_vec_string() {
    let theme = api_client()
        .themes()
        .retrieve_with_view_context(&THEME_TWENTY_TWENTY_FOUR.into())
        .await
        .assert_response()
        .data;
    assert!(matches!(
        theme
            .theme_supports
            .expect("'twentytwentyfour' theme includes the 'theme_supports' field")
            .get(&ThemeSupports::Html5)
            .expect("'twentytwentyfour' theme's 'theme_supports' field includes 'html5' type"),
        ThemeSupportsData::VecString(_)
    ));
}

#[template]
#[rstest]
#[case::default(ThemeListParams::default())]
#[case::status_active(generate!(ThemeListParams, (status, Some(ThemeStatus::Active))))]
#[case::status_inactive(generate!(ThemeListParams, (status, Some(ThemeStatus::Inactive))))]
pub fn list_cases(#[case] params: ThemeListParams) {}

#[template]
#[rstest]
#[case::twentytwentyfive(THEME_TWENTY_TWENTY_FIVE.into())]
#[case::twentytwentyfour(THEME_TWENTY_TWENTY_FOUR.into())]
#[case::twentytwentythree(THEME_TWENTY_TWENTY_THREE.into())]
pub fn retrieve_cases(#[case] stylesheet: ThemeStylesheet) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_theme_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_theme_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_theme_field_with_view_context_test_cases!();

    #[apply(sparse_theme_field_with_edit_context_test_cases)]
    #[case(&[SparseThemeFieldWithEditContext::Stylesheet, SparseThemeFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseThemeFieldWithEditContext],
        #[values(
            ThemeListParams::default(),
            generate!(ThemeListParams, (status, Some(ThemeStatus::Active))),
            generate!(ThemeListParams, (status, Some(ThemeStatus::Inactive)))
        )]
        params: ThemeListParams,
    ) {
        api_client()
            .themes()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|theme| {
                theme.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_theme_field_with_edit_context_test_cases)]
    #[case(&[SparseThemeFieldWithEditContext::Stylesheet, SparseThemeFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseThemeFieldWithEditContext],
        #[values(
            THEME_TWENTY_TWENTY_FIVE,
            THEME_TWENTY_TWENTY_FOUR,
            THEME_TWENTY_TWENTY_THREE
        )]
        stylesheet: &str,
    ) {
        let theme = api_client()
            .themes()
            .filter_retrieve_with_edit_context(&stylesheet.into(), fields)
            .await
            .assert_response()
            .data;
        theme.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_theme_field_with_embed_context_test_cases)]
    #[case(&[SparseThemeFieldWithEmbedContext::Stylesheet, SparseThemeFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseThemeFieldWithEmbedContext],
        #[values(
            ThemeListParams::default(),
            generate!(ThemeListParams, (status, Some(ThemeStatus::Active))),
            generate!(ThemeListParams, (status, Some(ThemeStatus::Inactive)))
        )]
        params: ThemeListParams,
    ) {
        api_client()
            .themes()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|theme| {
                theme.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_theme_field_with_embed_context_test_cases)]
    #[case(&[SparseThemeFieldWithEmbedContext::Stylesheet, SparseThemeFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseThemeFieldWithEmbedContext],
        #[values(
            THEME_TWENTY_TWENTY_FIVE,
            THEME_TWENTY_TWENTY_FOUR,
            THEME_TWENTY_TWENTY_THREE
        )]
        stylesheet: &str,
    ) {
        let theme = api_client()
            .themes()
            .filter_retrieve_with_embed_context(&stylesheet.into(), fields)
            .await
            .assert_response()
            .data;
        theme.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_theme_field_with_view_context_test_cases)]
    #[case(&[SparseThemeFieldWithViewContext::Stylesheet, SparseThemeFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseThemeFieldWithViewContext],
        #[values(
            ThemeListParams::default(),
            generate!(ThemeListParams, (status, Some(ThemeStatus::Active))),
            generate!(ThemeListParams, (status, Some(ThemeStatus::Inactive)))
        )]
        params: ThemeListParams,
    ) {
        api_client()
            .themes()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|theme| {
                theme.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_theme_field_with_view_context_test_cases)]
    #[case(&[SparseThemeFieldWithViewContext::Stylesheet, SparseThemeFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseThemeFieldWithViewContext],
        #[values(
            THEME_TWENTY_TWENTY_FIVE,
            THEME_TWENTY_TWENTY_FOUR,
            THEME_TWENTY_TWENTY_THREE
        )]
        stylesheet: &str,
    ) {
        let theme = api_client()
            .themes()
            .filter_retrieve_with_view_context(&stylesheet.into(), fields)
            .await
            .assert_response()
            .data;
        theme.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}
