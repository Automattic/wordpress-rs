use wp_api::{
    WpApiParamOrder,
    nav_menus::{
        NavMenuId, NavMenuListParams, SparseNavMenuFieldWithEditContext,
        SparseNavMenuFieldWithEmbedContext, SparseNavMenuFieldWithViewContext,
        WpApiParamNavMenusOrderBy,
    },
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: NavMenuListParams) {
    api_client()
        .nav_menus()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: NavMenuListParams) {
    api_client()
        .nav_menus()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: NavMenuListParams) {
    api_client()
        .nav_menus()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .nav_menus()
        .retrieve_with_edit_context(&NAV_MENU_ID_179)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .nav_menus()
        .retrieve_with_embed_context(&NAV_MENU_ID_179)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .nav_menus()
        .retrieve_with_view_context(&NAV_MENU_ID_179)
        .await
        .assert_response();
}

#[template]
#[rstest]
#[case::default(NavMenuListParams::default())]
#[case::page(generate!(NavMenuListParams, (page, Some(1))))]
#[case::per_page(generate!(NavMenuListParams, (per_page, Some(3))))]
#[case::search(generate!(NavMenuListParams, (search, Some("menu".to_string()))))]
#[case::exclude(generate!(NavMenuListParams, (exclude, vec![NavMenuId(99999)])))]
#[case::include(generate!(NavMenuListParams, (include, vec![NavMenuId(1)])))]
#[case::offset(generate!(NavMenuListParams, (offset, Some(1))))]
#[case::order(generate!(NavMenuListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Id))))]
#[case::include(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Include))))]
#[case::orderby_name(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Name))))]
#[case::orderby_slug(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Slug))))]
#[case::include_slugs(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::IncludeSlugs))))]
#[case::term_group(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::TermGroup))))]
#[case::orderby_description(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Description))))]
#[case::orderby_count(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Count))))]
#[case::hide_empty(generate!(NavMenuListParams, (hide_empty, Some(false))))]
#[case::post(generate!(NavMenuListParams, (post, Some(POST_ID_NAV_MENUS_PARAM))))]
#[case::slug(generate!(NavMenuListParams, (slug, vec!["primary".to_string(), "footer".to_string()])))]
fn list_cases(#[case] params: NavMenuListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_nav_menu_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_nav_menu_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_nav_menu_field_with_view_context_test_cases!();

    #[apply(sparse_nav_menu_field_with_edit_context_test_cases)]
    #[case(&[SparseNavMenuFieldWithEditContext::Id, SparseNavMenuFieldWithEditContext::Name])]
    #[tokio::test]
    #[parallel]
    async fn filter_nav_menus_with_edit_context(
        #[case] fields: &[SparseNavMenuFieldWithEditContext],
        #[values(
            NavMenuListParams::default(),
            generate!(NavMenuListParams, (search, Some("menu".to_string()))),
            generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Name)))
        )]
        params: NavMenuListParams,
    ) {
        let nav_menus = api_client()
            .nav_menus()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .data;

        nav_menus.iter().for_each(|nav_menu| {
            nav_menu.assert_that_instance_fields_nullability_match_provided_fields(fields)
        });
    }

    #[apply(sparse_nav_menu_field_with_edit_context_test_cases)]
    #[case(&[SparseNavMenuFieldWithEditContext::Id, SparseNavMenuFieldWithEditContext::Name])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_nav_menus_with_edit_context(
        #[case] fields: &[SparseNavMenuFieldWithEditContext],
    ) {
        let nav_menu = api_client()
            .nav_menus()
            .filter_retrieve_with_edit_context(&NAV_MENU_ID_179, fields)
            .await
            .assert_response()
            .data;
        nav_menu.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_nav_menu_field_with_embed_context_test_cases)]
    #[case(&[SparseNavMenuFieldWithEmbedContext::Id, SparseNavMenuFieldWithEmbedContext::Name])]
    #[tokio::test]
    #[parallel]
    async fn filter_nav_menus_with_embed_context(
        #[case] fields: &[SparseNavMenuFieldWithEmbedContext],
        #[values(
            NavMenuListParams::default(),
            generate!(NavMenuListParams, (search, Some("menu".to_string()))),
            generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Name)))
        )]
        params: NavMenuListParams,
    ) {
        let nav_menus = api_client()
            .nav_menus()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data;

        nav_menus.iter().for_each(|nav_menu| {
            nav_menu.assert_that_instance_fields_nullability_match_provided_fields(fields)
        });
    }

    #[apply(sparse_nav_menu_field_with_embed_context_test_cases)]
    #[case(&[SparseNavMenuFieldWithEmbedContext::Id, SparseNavMenuFieldWithEmbedContext::Name])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_nav_menus_with_embed_context(
        #[case] fields: &[SparseNavMenuFieldWithEmbedContext],
    ) {
        let nav_menu = api_client()
            .nav_menus()
            .filter_retrieve_with_embed_context(&NAV_MENU_ID_179, fields)
            .await
            .assert_response()
            .data;
        nav_menu.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_nav_menu_field_with_view_context_test_cases)]
    #[case(&[SparseNavMenuFieldWithViewContext::Id, SparseNavMenuFieldWithViewContext::Name])]
    #[tokio::test]
    #[parallel]
    async fn filter_nav_menus_with_view_context(
        #[case] fields: &[SparseNavMenuFieldWithViewContext],
        #[values(
            NavMenuListParams::default(),
            generate!(NavMenuListParams, (search, Some("menu".to_string()))),
            generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Name)))
        )]
        params: NavMenuListParams,
    ) {
        let nav_menus = api_client()
            .nav_menus()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data;

        nav_menus.iter().for_each(|nav_menu| {
            nav_menu.assert_that_instance_fields_nullability_match_provided_fields(fields)
        });
    }

    #[apply(sparse_nav_menu_field_with_view_context_test_cases)]
    #[case(&[SparseNavMenuFieldWithViewContext::Id, SparseNavMenuFieldWithViewContext::Name])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_nav_menus_with_view_context(
        #[case] fields: &[SparseNavMenuFieldWithViewContext],
    ) {
        let nav_menu = api_client()
            .nav_menus()
            .filter_retrieve_with_view_context(&NAV_MENU_ID_179, fields)
            .await
            .assert_response()
            .data;
        nav_menu.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}
