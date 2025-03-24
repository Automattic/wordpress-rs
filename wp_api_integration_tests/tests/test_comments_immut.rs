use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::comments::{
    CommentId, CommentListParams, CommentRetrieveParams, CommentStatus, CommentType,
    SparseCommentFieldWithEditContext, SparseCommentFieldWithEmbedContext,
    SparseCommentFieldWithViewContext, WpApiParamCommentsOrderBy,
};
use wp_api::posts::PostId;
use wp_api::users::UserAvatarSize;
use wp_api::{WpApiParamOrder, generate};
use wp_api_integration_tests::{
    AssertResponse, FIRST_COMMENT_ID, FIRST_USER_EMAIL, FIRST_USER_ID, SECOND_USER_ID,
    TestCredentials, api_client, unwrapped_wp_gmt_date_time,
};

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: CommentListParams) {
    api_client()
        .comments()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: CommentListParams) {
    api_client()
        .comments()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: CommentListParams) {
    api_client()
        .comments()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .comments()
        .retrieve_with_edit_context(&FIRST_COMMENT_ID, &CommentRetrieveParams::default())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context(#[case] params: CommentRetrieveParams) {
    api_client()
        .comments()
        .retrieve_with_embed_context(&FIRST_COMMENT_ID, &CommentRetrieveParams::default())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context(#[case] params: CommentRetrieveParams) {
    api_client()
        .comments()
        .retrieve_with_view_context(&FIRST_COMMENT_ID, &CommentRetrieveParams::default())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_with_edit_context() {
    let test_credentials = TestCredentials::instance();
    let comment = api_client()
        .comments()
        .retrieve_with_edit_context(
            &CommentId(test_credentials.password_protected_comment_id),
            &CommentRetrieveParams {
                password: Some(
                    test_credentials
                        .password_protected_post_password
                        .to_string(),
                ),
            },
        )
        .await
        .assert_response()
        .data;
    assert_eq!(
        comment.author_name,
        test_credentials.password_protected_comment_author
    );
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_with_embed_context() {
    let test_credentials = TestCredentials::instance();
    let comment = api_client()
        .comments()
        .retrieve_with_embed_context(
            &CommentId(test_credentials.password_protected_comment_id),
            &CommentRetrieveParams {
                password: Some(
                    test_credentials
                        .password_protected_post_password
                        .to_string(),
                ),
            },
        )
        .await
        .assert_response()
        .data;
    assert_eq!(
        comment.author_name,
        test_credentials.password_protected_comment_author
    );
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_with_view_context() {
    let test_credentials = TestCredentials::instance();
    let comment = api_client()
        .comments()
        .retrieve_with_view_context(
            &CommentId(test_credentials.password_protected_comment_id),
            &CommentRetrieveParams {
                password: Some(
                    test_credentials
                        .password_protected_post_password
                        .to_string(),
                ),
            },
        )
        .await
        .assert_response()
        .data;
    assert_eq!(
        comment.author_name,
        test_credentials.password_protected_comment_author
    );
}

#[tokio::test]
#[rstest]
#[parallel]
#[case(CommentListParams { per_page: Some(1), ..Default::default() })]
#[case(CommentListParams { per_page: Some(1), order: Some(WpApiParamOrder::Desc), ..Default::default() })]
#[case(CommentListParams { per_page: Some(1), orderby: Some(WpApiParamCommentsOrderBy::Id), ..Default::default() })]
async fn paginate_list_comments_with_edit_context(#[case] params: CommentListParams) {
    let first_page_response = api_client()
        .comments()
        .list_with_edit_context(&params)
        .await
        .assert_response();
    assert!(!first_page_response.data.is_empty());
    let next_page_params = first_page_response.next_page_params.unwrap();
    let next_page_response = api_client()
        .comments()
        .list_with_edit_context(&next_page_params)
        .await
        .assert_response();
    assert!(!next_page_response.data.is_empty());
    let prev_page_params = next_page_response.prev_page_params.unwrap();
    let prev_page_response = api_client()
        .comments()
        .list_with_edit_context(&prev_page_params)
        .await
        .assert_response();
    assert!(!prev_page_response.data.is_empty());
}

#[tokio::test]
#[rstest]
#[parallel]
#[case(true, CommentListParams { comment_type: Some(CommentType::Comment), ..Default::default() })]
#[case(false, CommentListParams { comment_type: Some(CommentType::Pingback), ..Default::default() })]
#[case(false, CommentListParams { comment_type: Some(CommentType::Trackback), ..Default::default() })]
async fn list_comments_with_edit_context_parse_author_avatar_urls(
    #[case] size_24_included: bool,
    #[case] params: CommentListParams,
) {
    api_client()
        .comments()
        .list_with_view_context(&params)
        .await
        .assert_response()
        .data
        .into_iter()
        .for_each(|mut c| {
            assert_eq!(
                size_24_included,
                c.author_avatar_urls
                    .remove(&UserAvatarSize::Size24)
                    .unwrap()
                    .0
                    .is_some(),
                "{:#?}",
                c.author_avatar_urls
            )
        });
}

#[template]
#[rstest]
#[case::default(CommentListParams::default())]
#[case::page(generate!(CommentListParams, (page, Some(1))))]
#[case::per_page(generate!(CommentListParams, (per_page, Some(3))))]
#[case::search(generate!(CommentListParams, (search, Some("foo".to_string()))))]
#[case::after(generate!(CommentListParams, (after, Some(unwrapped_wp_gmt_date_time("2020-08-14T17:00:00+0200")))))]
#[case::author(generate!(CommentListParams, (author, vec![FIRST_USER_ID, SECOND_USER_ID])))]
#[case::author_exclude(generate!(CommentListParams, (author_exclude, vec![SECOND_USER_ID])))]
#[case::author_email(generate!(CommentListParams, (author_email, Some(FIRST_USER_EMAIL.to_string()))))]
#[case::before(generate!(CommentListParams, (before, Some(unwrapped_wp_gmt_date_time("2023-08-14T17:00:00+0000")))))]
#[case::exclude(generate!(CommentListParams, (exclude, vec![CommentId(1), CommentId(2)])))]
#[case::include(generate!(CommentListParams, (include, vec![CommentId(1)])))]
#[case::offset(generate!(CommentListParams, (offset, Some(2))))]
#[case::order(generate!(CommentListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Id))))]
#[case::parent(generate!(CommentListParams, (parent, vec![CommentId(1), CommentId(2)])))]
#[case::parent_exclude(generate!(CommentListParams, (parent, vec![CommentId(1), CommentId(2)])))]
#[case::post(generate!(CommentListParams, (post, vec![PostId(1), PostId(2)])))]
#[case::status_hold(generate!(CommentListParams, (status, Some(CommentStatus::Hold))))]
#[case::status_approve(generate!(CommentListParams, (status, Some(CommentStatus::Approve))))]
#[case::status_spam(generate!(CommentListParams, (status, Some(CommentStatus::Spam))))]
#[case::status_trash(generate!(CommentListParams, (status, Some(CommentStatus::Trash))))]
#[case::comment_type_comment(generate!(CommentListParams, (comment_type, Some(CommentType::Comment))))]
#[case::comment_type_pingback(generate!(CommentListParams, (comment_type, Some(CommentType::Pingback))))]
#[case::comment_type_trackback(generate!(CommentListParams, (comment_type, Some(CommentType::Trackback))))]
#[case::password(generate!(CommentListParams, (password, Some("foo".to_string()))))]
pub fn list_cases(#[case] params: CommentListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_comment_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_comment_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_comment_field_with_view_context_test_cases!();

    #[apply(sparse_comment_field_with_edit_context_test_cases)]
    #[case(&[SparseCommentFieldWithEditContext::Id, SparseCommentFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_comments_with_edit_context(
        #[case] fields: &[SparseCommentFieldWithEditContext],
        #[values(
            CommentListParams::default(),
            generate!(CommentListParams, (page, Some(2))),
            generate!(CommentListParams, (search, Some("foo".to_string())))
        )]
        params: CommentListParams,
    ) {
        api_client()
            .comments()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|comment| {
                comment.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_comment_field_with_edit_context_test_cases)]
    #[case(&[SparseCommentFieldWithEditContext::Id, SparseCommentFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_comments_with_edit_context(
        #[case] fields: &[SparseCommentFieldWithEditContext],
    ) {
        let comment = api_client()
            .comments()
            .filter_retrieve_with_edit_context(
                &FIRST_COMMENT_ID,
                &CommentRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        comment.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_comment_field_with_embed_context_test_cases)]
    #[case(&[SparseCommentFieldWithEmbedContext::Id, SparseCommentFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_comments_with_embed_context(
        #[case] fields: &[SparseCommentFieldWithEmbedContext],
        #[values(
            CommentListParams::default(),
            generate!(CommentListParams, (page, Some(2))),
            generate!(CommentListParams, (search, Some("foo".to_string())))
        )]
        params: CommentListParams,
    ) {
        api_client()
            .comments()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|comment| {
                comment.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_comment_field_with_embed_context_test_cases)]
    #[case(&[SparseCommentFieldWithEmbedContext::Id, SparseCommentFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_comments_with_embed_context(
        #[case] fields: &[SparseCommentFieldWithEmbedContext],
    ) {
        let comment = api_client()
            .comments()
            .filter_retrieve_with_embed_context(
                &FIRST_COMMENT_ID,
                &CommentRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        comment.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_comment_field_with_view_context_test_cases)]
    #[case(&[SparseCommentFieldWithViewContext::Id, SparseCommentFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_comments_with_view_context(
        #[case] fields: &[SparseCommentFieldWithViewContext],
        #[values(
            CommentListParams::default(),
            generate!(CommentListParams, (page, Some(2))),
            generate!(CommentListParams, (search, Some("foo".to_string())))
        )]
        params: CommentListParams,
    ) {
        api_client()
            .comments()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|comment| {
                comment.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_comment_field_with_view_context_test_cases)]
    #[case(&[SparseCommentFieldWithViewContext::Id, SparseCommentFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_comments_with_view_context(
        #[case] fields: &[SparseCommentFieldWithViewContext],
    ) {
        let comment = api_client()
            .comments()
            .filter_retrieve_with_view_context(
                &FIRST_COMMENT_ID,
                &CommentRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        comment.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}
