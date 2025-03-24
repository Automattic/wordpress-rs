use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::{
    WpApiParamOrder, generate,
    media::{
        MediaId, MediaListParams, MediaStatus, MediaTypeParam, SparseMediaFieldWithEditContext,
        SparseMediaFieldWithEmbedContext, SparseMediaFieldWithViewContext,
    },
    posts::{PostId, WpApiParamPostsOrderBy, WpApiParamPostsSearchColumn},
};
use wp_api_integration_tests::{
    AssertResponse, FIRST_USER_ID, MEDIA_ID_611, SECOND_USER_ID, api_client,
    unwrapped_wp_gmt_date_time,
};

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: MediaListParams) {
    api_client()
        .media()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: MediaListParams) {
    api_client()
        .media()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: MediaListParams) {
    api_client()
        .media()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .media()
        .retrieve_with_edit_context(&MEDIA_ID_611)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .media()
        .retrieve_with_embed_context(&MEDIA_ID_611)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .media()
        .retrieve_with_view_context(&MEDIA_ID_611)
        .await
        .assert_response();
}

#[tokio::test]
#[rstest]
#[parallel]
#[case(MediaListParams { per_page: Some(1), ..Default::default() })]
#[case(MediaListParams { per_page: Some(1), order: Some(WpApiParamOrder::Desc), ..Default::default() })]
#[case(MediaListParams { per_page: Some(1), orderby: Some(WpApiParamPostsOrderBy::Modified), ..Default::default() })]
async fn paginate_list_media_with_edit_context(#[case] params: MediaListParams) {
    let first_page_response = api_client()
        .media()
        .list_with_edit_context(&params)
        .await
        .assert_response();
    assert!(!first_page_response.data.is_empty());
    let next_page_params = first_page_response.next_page_params.unwrap();
    let next_page_response = api_client()
        .media()
        .list_with_edit_context(&next_page_params)
        .await
        .assert_response();
    assert!(!next_page_response.data.is_empty());
    let prev_page_params = next_page_response.prev_page_params.unwrap();
    let prev_page_response = api_client()
        .media()
        .list_with_edit_context(&prev_page_params)
        .await
        .assert_response();
    assert!(!prev_page_response.data.is_empty());
}

#[template]
#[rstest]
#[case::default(MediaListParams::default())]
#[case::page(generate!(MediaListParams, (page, Some(1))))]
#[case::per_page(generate!(MediaListParams, (per_page, Some(3))))]
#[case::search(generate!(MediaListParams, (search, Some("foo".to_string()))))]
#[case::after(generate!(MediaListParams, (after, Some(unwrapped_wp_gmt_date_time("2020-08-14T17:00:00+0200")))))]
#[case::modified_after(generate!(MediaListParams, (modified_after, Some(unwrapped_wp_gmt_date_time("2024-01-14T17:00:00+0200")))))]
#[case::author(generate!(MediaListParams, (author, vec![FIRST_USER_ID, SECOND_USER_ID])))]
#[case::author_exclude(generate!(MediaListParams, (author_exclude, vec![SECOND_USER_ID])))]
#[case::before(generate!(MediaListParams, (before, Some(unwrapped_wp_gmt_date_time("2023-08-14T17:00:00+0000")))))]
#[case::modified_before(generate!(MediaListParams, (modified_before, Some(unwrapped_wp_gmt_date_time("2024-01-14T17:00:00+0000")))))]
#[case::exclude(generate!(MediaListParams, (exclude, vec![MediaId(1), MediaId(2)])))]
#[case::include(generate!(MediaListParams, (include, vec![MediaId(1)])))]
#[case::offset(generate!(MediaListParams, (offset, Some(2))))]
#[case::order(generate!(MediaListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Id))))]
#[case::parent(generate!(MediaListParams, (parent, vec![PostId(1), PostId(2)])))]
#[case::parent_exclude(generate!(MediaListParams, (parent, vec![PostId(1), PostId(2)])))]
#[case::search_columns(generate!(MediaListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent, WpApiParamPostsSearchColumn::PostExcerpt])))]
#[case::slug(generate!(MediaListParams, (slug, vec!["foo".to_string(), "bar".to_string()])))]
#[case::status(generate!(MediaListParams, (status, vec![MediaStatus::Inherit, MediaStatus::Private, MediaStatus::Trash])))]
#[case::media_type(generate!(MediaListParams, (media_type, Some(MediaTypeParam::Image))))]
#[case::mime_type(generate!(MediaListParams, (mime_type, Some("image/jpeg".to_string()))))]
pub fn list_cases(#[case] params: MediaListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_media_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_media_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_media_field_with_view_context_test_cases!();

    #[apply(sparse_media_field_with_edit_context_test_cases)]
    #[case(&[SparseMediaFieldWithEditContext::Id, SparseMediaFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_media_with_edit_context(
        #[case] fields: &[SparseMediaFieldWithEditContext],
        #[values(
            MediaListParams::default(),
            generate!(MediaListParams, (page, Some(2))),
            generate!(MediaListParams, (search, Some("foo".to_string())))
        )]
        params: MediaListParams,
    ) {
        api_client()
            .media()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|media| {
                media.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_media_field_with_edit_context_test_cases)]
    #[case(&[SparseMediaFieldWithEditContext::Id, SparseMediaFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_media_with_edit_context(
        #[case] fields: &[SparseMediaFieldWithEditContext],
    ) {
        let media = api_client()
            .media()
            .filter_retrieve_with_edit_context(&MEDIA_ID_611, fields)
            .await
            .assert_response()
            .data;
        media.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_media_field_with_embed_context_test_cases)]
    #[case(&[SparseMediaFieldWithEmbedContext::Id, SparseMediaFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_media_with_embed_context(
        #[case] fields: &[SparseMediaFieldWithEmbedContext],
        #[values(
            MediaListParams::default(),
            generate!(MediaListParams, (page, Some(2))),
            generate!(MediaListParams, (search, Some("foo".to_string())))
        )]
        params: MediaListParams,
    ) {
        api_client()
            .media()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|media| {
                media.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_media_field_with_embed_context_test_cases)]
    #[case(&[SparseMediaFieldWithEmbedContext::Id, SparseMediaFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_media_with_embed_context(
        #[case] fields: &[SparseMediaFieldWithEmbedContext],
    ) {
        let media = api_client()
            .media()
            .filter_retrieve_with_embed_context(&MEDIA_ID_611, fields)
            .await
            .assert_response()
            .data;
        media.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_media_field_with_view_context_test_cases)]
    #[case(&[SparseMediaFieldWithViewContext::Id, SparseMediaFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_media_with_view_context(
        #[case] fields: &[SparseMediaFieldWithViewContext],
        #[values(
            MediaListParams::default(),
            generate!(MediaListParams, (page, Some(2))),
            generate!(MediaListParams, (search, Some("foo".to_string())))
        )]
        params: MediaListParams,
    ) {
        api_client()
            .media()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|media| {
                media.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_media_field_with_view_context_test_cases)]
    #[case(&[SparseMediaFieldWithViewContext::Id, SparseMediaFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_media_with_view_context(
        #[case] fields: &[SparseMediaFieldWithViewContext],
    ) {
        let media = api_client()
            .media()
            .filter_retrieve_with_view_context(&MEDIA_ID_611, fields)
            .await
            .assert_response()
            .data;
        media.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}
