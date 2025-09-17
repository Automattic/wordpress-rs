use wp_api::{
    pages::{
        PageId, PageListParams, PageRetrieveParams, PageStatus, SparsePageFieldWithEditContext,
        SparsePageFieldWithEmbedContext, SparsePageFieldWithViewContext, WpApiParamPagesOrderBy,
    },
    posts::WpApiParamPostsSearchColumn,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: PageListParams) {
    api_client()
        .pages()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: PageListParams) {
    api_client()
        .pages()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: PageListParams) {
    api_client()
        .pages()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    let test_credentials = TestCredentials::instance();
    api_client()
        .pages()
        .retrieve_with_edit_context(
            &PageId(test_credentials.first_page_id),
            &PageRetrieveParams::default(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context(#[case] params: PageRetrieveParams) {
    let test_credentials = TestCredentials::instance();
    api_client()
        .pages()
        .retrieve_with_embed_context(
            &PageId(test_credentials.first_page_id),
            &PageRetrieveParams::default(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context(#[case] params: PageRetrieveParams) {
    let test_credentials = TestCredentials::instance();
    api_client()
        .pages()
        .retrieve_with_view_context(
            &PageId(test_credentials.first_page_id),
            &PageRetrieveParams::default(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_with_edit_context() {
    let test_credentials = TestCredentials::instance();
    let page = api_client()
        .pages()
        .retrieve_with_edit_context(
            &PageId(test_credentials.password_protected_page_id),
            &PageRetrieveParams {
                password: Some(
                    test_credentials
                        .password_protected_page_password
                        .to_string(),
                ),
            },
        )
        .await
        .assert_response()
        .data;
    assert_eq!(
        page.title.rendered,
        test_credentials.password_protected_page_title
    );
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_with_embed_context() {
    let test_credentials = TestCredentials::instance();
    let page = api_client()
        .pages()
        .retrieve_with_embed_context(
            &PageId(test_credentials.password_protected_page_id),
            &PageRetrieveParams {
                password: Some(
                    test_credentials
                        .password_protected_page_password
                        .to_string(),
                ),
            },
        )
        .await
        .assert_response()
        .data;
    assert_eq!(
        page.title.rendered,
        test_credentials.password_protected_page_title
    );
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_with_view_context() {
    let test_credentials = TestCredentials::instance();
    let page = api_client()
        .pages()
        .retrieve_with_view_context(
            &PageId(test_credentials.password_protected_page_id),
            &PageRetrieveParams {
                password: Some(
                    test_credentials
                        .password_protected_page_password
                        .to_string(),
                ),
            },
        )
        .await
        .assert_response()
        .data;
    assert_eq!(
        page.title.rendered,
        test_credentials.password_protected_page_title
    );
}

#[tokio::test]
#[rstest]
#[parallel]
#[case(PageListParams { per_page: Some(1), ..Default::default() })]
#[case(PageListParams { per_page: Some(1), order: Some(WpApiParamOrder::Desc), ..Default::default() })]
#[case(PageListParams { per_page: Some(1), orderby: Some(WpApiParamPagesOrderBy::Modified), ..Default::default() })]
async fn paginate_list_pages_with_edit_context(#[case] params: PageListParams) {
    let first_page_response = api_client()
        .pages()
        .list_with_edit_context(&params)
        .await
        .assert_response();
    assert!(!first_page_response.data.is_empty());
    let next_page_params = first_page_response.next_page_params.unwrap();
    let next_page_response = api_client()
        .pages()
        .list_with_edit_context(&next_page_params)
        .await
        .assert_response();
    assert!(!next_page_response.data.is_empty());
    let prev_page_params = next_page_response.prev_page_params.unwrap();
    let prev_page_response = api_client()
        .pages()
        .list_with_edit_context(&prev_page_params)
        .await
        .assert_response();
    assert!(!prev_page_response.data.is_empty());
}

#[template]
#[rstest]
#[case::default(PageListParams::default())]
#[case::page(generate!(PageListParams, (page, Some(1))))]
#[case::per_page(generate!(PageListParams, (per_page, Some(3))))]
#[case::search(generate!(PageListParams, (search, Some("foo".to_string()))))]
#[case::after(generate!(PageListParams, (after, Some(unwrapped_wp_gmt_date_time("2020-08-14T17:00:00+0200")))))]
#[case::modified_after(generate!(PageListParams, (modified_after, Some(unwrapped_wp_gmt_date_time("2024-01-14T17:00:00+0200")))))]
#[case::author(generate!(PageListParams, (author, vec![FIRST_USER_ID, SECOND_USER_ID])))]
#[case::author_exclude(generate!(PageListParams, (author_exclude, vec![SECOND_USER_ID])))]
#[case::before(generate!(PageListParams, (before, Some(unwrapped_wp_gmt_date_time("2023-08-14T17:00:00+0000")))))]
#[case::modified_before(generate!(PageListParams, (modified_before, Some(unwrapped_wp_gmt_date_time("2024-01-14T17:00:00+0000")))))]
#[case::exclude(generate!(PageListParams, (exclude, vec![PageId(1), PageId(2)])))]
#[case::include(generate!(PageListParams, (include, vec![PageId(1)])))]
#[case::offset(generate!(PageListParams, (offset, Some(2))))]
#[case::order(generate!(PageListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Id))))]
#[case::search_columns(generate!(PageListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent, WpApiParamPostsSearchColumn::PostExcerpt])))]
#[case::slug(generate!(PageListParams, (slug, vec!["foo".to_string(), "bar".to_string()])))]
#[case::status(generate!(PageListParams, (status, vec![PageStatus::Publish, PageStatus::Pending])))]
#[case::parent(generate!(PageListParams, (parent, Some(PageId(1)))))]
#[case::parent_exclude(generate!(PageListParams, (parent_exclude, vec![PageId(1), PageId(2)])))]
#[case::menu_order(generate!(PageListParams, (menu_order, Some(1))))]
#[case::orderby_menu_order(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::MenuOrder))))]
#[case::orderby_parent(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Parent))))]
fn list_cases(#[case] params: PageListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_page_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_page_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_page_field_with_view_context_test_cases!();

    #[apply(sparse_page_field_with_edit_context_test_cases)]
    #[case(&[SparsePageFieldWithEditContext::Id, SparsePageFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_pages_with_edit_context(
        #[case] fields: &[SparsePageFieldWithEditContext],
        #[values(
            PageListParams::default(),
            generate!(PageListParams, (status, vec![PageStatus::Draft, PageStatus::Publish])),
            generate!(PageListParams, (search, Some("foo".to_string())))
        )]
        params: PageListParams,
    ) {
        api_client()
            .pages()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|page| {
                page.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_page_field_with_edit_context_test_cases)]
    #[case(&[SparsePageFieldWithEditContext::Id, SparsePageFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_pages_with_edit_context(
        #[case] fields: &[SparsePageFieldWithEditContext],
    ) {
        let test_credentials = TestCredentials::instance();
        let page = api_client()
            .pages()
            .filter_retrieve_with_edit_context(
                &PageId(test_credentials.first_page_id),
                &PageRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        page.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_page_field_with_embed_context_test_cases)]
    #[case(&[SparsePageFieldWithEmbedContext::Id, SparsePageFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_pages_with_embed_context(
        #[case] fields: &[SparsePageFieldWithEmbedContext],
        #[values(
            PageListParams::default(),
            generate!(PageListParams, (status, vec![PageStatus::Draft, PageStatus::Publish])),
            generate!(PageListParams, (search, Some("foo".to_string())))
        )]
        params: PageListParams,
    ) {
        api_client()
            .pages()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|page| {
                page.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_page_field_with_embed_context_test_cases)]
    #[case(&[SparsePageFieldWithEmbedContext::Id, SparsePageFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_pages_with_embed_context(
        #[case] fields: &[SparsePageFieldWithEmbedContext],
    ) {
        let test_credentials = TestCredentials::instance();
        let page = api_client()
            .pages()
            .filter_retrieve_with_embed_context(
                &PageId(test_credentials.first_page_id),
                &PageRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        page.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_page_field_with_view_context_test_cases)]
    #[case(&[SparsePageFieldWithViewContext::Id, SparsePageFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_pages_with_view_context(
        #[case] fields: &[SparsePageFieldWithViewContext],
        #[values(
            PageListParams::default(),
            generate!(PageListParams, (status, vec![PageStatus::Draft, PageStatus::Publish])),
            generate!(PageListParams, (search, Some("foo".to_string())))
        )]
        params: PageListParams,
    ) {
        api_client()
            .pages()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|page| {
                page.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_page_field_with_view_context_test_cases)]
    #[case(&[SparsePageFieldWithViewContext::Id, SparsePageFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_pages_with_view_context(
        #[case] fields: &[SparsePageFieldWithViewContext],
    ) {
        let test_credentials = TestCredentials::instance();
        let page = api_client()
            .pages()
            .filter_retrieve_with_view_context(
                &PageId(test_credentials.first_page_id),
                &PageRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        page.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}
