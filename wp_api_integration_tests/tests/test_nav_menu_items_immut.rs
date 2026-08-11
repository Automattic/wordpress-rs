use wp_api::date::WpDateString;
use wp_api::{
    WpApiParamOrder,
    nav_menu_items::{
        NavMenuItemId, NavMenuItemListParams, NavMenuItemStatus,
        SparseNavMenuItemFieldWithEditContext, SparseNavMenuItemFieldWithEmbedContext,
        SparseNavMenuItemFieldWithViewContext,
    },
    posts::{WpApiParamPostsOrderBy, WpApiParamPostsSearchColumn, WpApiParamPostsTaxRelation},
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: NavMenuItemListParams) {
    api_client()
        .nav_menu_items()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: NavMenuItemListParams) {
    api_client()
        .nav_menu_items()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: NavMenuItemListParams) {
    api_client()
        .nav_menu_items()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .nav_menu_items()
        .retrieve_with_edit_context(&nav_menu_item_id_for_retrieve_tests())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .nav_menu_items()
        .retrieve_with_embed_context(&nav_menu_item_id_for_retrieve_tests())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .nav_menu_items()
        .retrieve_with_view_context(&nav_menu_item_id_for_retrieve_tests())
        .await
        .assert_response();
}

#[tokio::test]
#[rstest]
#[parallel]
#[case(NavMenuItemListParams { per_page: Some(1), ..Default::default() })]
async fn paginate_list_nav_menu_items_with_edit_context(#[case] params: NavMenuItemListParams) {
    let first_page_response = api_client()
        .nav_menu_items()
        .list_with_edit_context(&params)
        .await
        .assert_response();
    assert!(!first_page_response.data.is_empty());
    let next_page_params = first_page_response.next_page_params.unwrap();
    let next_page_response = api_client()
        .nav_menu_items()
        .list_with_edit_context(&next_page_params)
        .await
        .assert_response();
    assert!(!next_page_response.data.is_empty());
    let prev_page_params = next_page_response.prev_page_params.unwrap();
    let prev_page_response = api_client()
        .nav_menu_items()
        .list_with_edit_context(&prev_page_params)
        .await
        .assert_response();
    assert!(!prev_page_response.data.is_empty());
}

fn nav_menu_item_id_for_retrieve_tests() -> NavMenuItemId {
    NavMenuItemId(TestCredentials::instance().nav_menu_item_id)
}

#[template]
#[rstest]
#[case::default(NavMenuItemListParams::default())]
#[case::per_page(generate!(NavMenuItemListParams, (per_page, Some(5))))]
#[case::search(generate!(NavMenuItemListParams, (search, Some("test".to_string()))))]
#[case::after(generate!(NavMenuItemListParams, (after, Some(WpDateString("2020-08-14T17:00:00".to_string())))))]
#[case::modified_after(generate!(NavMenuItemListParams, (modified_after, Some(WpDateString("2024-01-14T17:00:00".to_string())))))]
#[case::before(generate!(NavMenuItemListParams, (before, Some(WpDateString("2023-08-14T17:00:00".to_string())))))]
#[case::modified_before(generate!(NavMenuItemListParams, (modified_before, Some(WpDateString("2024-01-14T17:00:00".to_string())))))]
#[case::exclude(generate!(NavMenuItemListParams, (exclude, vec![NavMenuItemId(1), NavMenuItemId(2)])))]
#[case::include(generate!(NavMenuItemListParams, (include, vec![nav_menu_item_id_for_retrieve_tests()])))]
#[case::offset(generate!(NavMenuItemListParams, (offset, Some(2))))]
#[case::order(generate!(NavMenuItemListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::Id))))]
#[case::search_columns(generate!(NavMenuItemListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent, WpApiParamPostsSearchColumn::PostExcerpt])))]
#[case::slug(generate!(NavMenuItemListParams, (slug, vec!["foo".to_string(), "bar".to_string()])))]
#[case::status(generate!(NavMenuItemListParams, (status, vec![NavMenuItemStatus::Publish, NavMenuItemStatus::Draft])))]
#[case::tax_relation(generate!(NavMenuItemListParams, (tax_relation, Some(WpApiParamPostsTaxRelation::And))))]
#[case::menus(generate!(NavMenuItemListParams, (menus, vec![NAV_MENU_ID_179])))]
#[case::menus_exclude(generate!(NavMenuItemListParams, (menus_exclude, vec![NAV_MENU_ID_179])))]
#[case::menu_order(generate!(NavMenuItemListParams, (menu_order, Some(1))))]
pub fn list_cases(#[case] params: NavMenuItemListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_nav_menu_item_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_nav_menu_item_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_nav_menu_item_field_with_view_context_test_cases!();

    #[apply(sparse_nav_menu_item_field_with_edit_context_test_cases)]
    #[case(&[SparseNavMenuItemFieldWithEditContext::Id, SparseNavMenuItemFieldWithEditContext::TypeLabel])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseNavMenuItemFieldWithEditContext],
        #[values(
            NavMenuItemListParams::default(),
            generate!(NavMenuItemListParams, (per_page, Some(5))),
            generate!(NavMenuItemListParams, (order, Some(WpApiParamOrder::Desc))),
            generate!(NavMenuItemListParams, (menus, vec![NAV_MENU_ID_179])),
            generate!(NavMenuItemListParams, (status, vec![NavMenuItemStatus::Publish]))
        )]
        params: NavMenuItemListParams,
    ) {
        api_client()
            .nav_menu_items()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|nav_menu_item| {
                nav_menu_item.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_nav_menu_item_field_with_edit_context_test_cases)]
    #[case(&[SparseNavMenuItemFieldWithEditContext::Id, SparseNavMenuItemFieldWithEditContext::TypeLabel])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseNavMenuItemFieldWithEditContext],
    ) {
        let nav_menu_item = api_client()
            .nav_menu_items()
            .filter_retrieve_with_edit_context(&nav_menu_item_id_for_retrieve_tests(), fields)
            .await
            .assert_response()
            .data;
        nav_menu_item.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_nav_menu_item_field_with_embed_context_test_cases)]
    #[case(&[SparseNavMenuItemFieldWithEmbedContext::Id, SparseNavMenuItemFieldWithEmbedContext::TypeLabel])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseNavMenuItemFieldWithEmbedContext],
        #[values(
            NavMenuItemListParams::default(),
            generate!(NavMenuItemListParams, (per_page, Some(5))),
            generate!(NavMenuItemListParams, (order, Some(WpApiParamOrder::Desc))),
            generate!(NavMenuItemListParams, (menus, vec![NAV_MENU_ID_179])),
            generate!(NavMenuItemListParams, (status, vec![NavMenuItemStatus::Publish]))
        )]
        params: NavMenuItemListParams,
    ) {
        api_client()
            .nav_menu_items()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|nav_menu_item| {
                nav_menu_item.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_nav_menu_item_field_with_embed_context_test_cases)]
    #[case(&[SparseNavMenuItemFieldWithEmbedContext::Id, SparseNavMenuItemFieldWithEmbedContext::TypeLabel])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseNavMenuItemFieldWithEmbedContext],
    ) {
        let nav_menu_item = api_client()
            .nav_menu_items()
            .filter_retrieve_with_embed_context(&nav_menu_item_id_for_retrieve_tests(), fields)
            .await
            .assert_response()
            .data;
        nav_menu_item.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_nav_menu_item_field_with_view_context_test_cases)]
    #[case(&[SparseNavMenuItemFieldWithViewContext::Id, SparseNavMenuItemFieldWithViewContext::TypeLabel])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseNavMenuItemFieldWithViewContext],
        #[values(
            NavMenuItemListParams::default(),
            generate!(NavMenuItemListParams, (per_page, Some(5))),
            generate!(NavMenuItemListParams, (order, Some(WpApiParamOrder::Desc))),
            generate!(NavMenuItemListParams, (menus, vec![NAV_MENU_ID_179])),
            generate!(NavMenuItemListParams, (status, vec![NavMenuItemStatus::Publish]))
        )]
        params: NavMenuItemListParams,
    ) {
        api_client()
            .nav_menu_items()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|nav_menu_item| {
                nav_menu_item.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_nav_menu_item_field_with_view_context_test_cases)]
    #[case(&[SparseNavMenuItemFieldWithViewContext::Id, SparseNavMenuItemFieldWithViewContext::TypeLabel])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseNavMenuItemFieldWithViewContext],
    ) {
        let nav_menu_item = api_client()
            .nav_menu_items()
            .filter_retrieve_with_view_context(&nav_menu_item_id_for_retrieve_tests(), fields)
            .await
            .assert_response()
            .data;
        nav_menu_item.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}
