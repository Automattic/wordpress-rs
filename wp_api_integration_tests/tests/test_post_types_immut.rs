use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::post_types::{
    PostType, PostTypeSupports, SparsePostTypeDetailsFieldWithEditContext,
    SparsePostTypeDetailsFieldWithEmbedContext, SparsePostTypeDetailsFieldWithViewContext,
};

use wp_api_integration_tests::{api_client, AssertResponse};

#[rstest]
#[tokio::test]
#[parallel]
async fn list_post_types_with_edit_context() {
    api_client()
        .post_types()
        .list_with_edit_context()
        .await
        .assert_response();
}

#[rstest]
#[tokio::test]
#[parallel]
async fn list_post_types_with_embed_context() {
    api_client()
        .post_types()
        .list_with_embed_context()
        .await
        .assert_response();
}

#[rstest]
#[tokio::test]
#[parallel]
async fn list_post_types_with_view_context() {
    api_client()
        .post_types()
        .list_with_view_context()
        .await
        .assert_response();
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_post_types_with_edit_context(
    #[values(
        PostType::Post,
        PostType::Page,
        PostType::Attachment,
        PostType::NavMenuItem,
        PostType::WpBlock,
        PostType::WpTemplate,
        PostType::WpTemplatePart,
        PostType::WpNavigation,
        PostType::WpFontFamily,
        PostType::WpFontFace
    )]
    post_type: PostType,
) {
    let post_type = api_client()
        .post_types()
        .retrieve_with_edit_context(&post_type)
        .await
        .assert_response();
    // All post types in our current testing sites support `Title`, so we use this assertion
    // to verify that we are able to parse it properly.
    //
    // It's entirely possible that we might have more test sites in the future and some of their
    // post types might not support `Title` in which case it's perfectly fine to completely
    // remove this assertion.
    assert_eq!(
        post_type.supports.get(&PostTypeSupports::Title),
        Some(true).as_ref()
    );
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_post_types_with_embed_context(
    #[values(
        PostType::Post,
        PostType::Page,
        PostType::Attachment,
        PostType::NavMenuItem,
        PostType::WpBlock,
        PostType::WpTemplate,
        PostType::WpTemplatePart,
        PostType::WpNavigation,
        PostType::WpFontFamily,
        PostType::WpFontFace
    )]
    post_type: PostType,
) {
    api_client()
        .post_types()
        .retrieve_with_embed_context(&post_type)
        .await
        .assert_response();
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_post_types_with_view_context(
    #[values(
        PostType::Post,
        PostType::Page,
        PostType::Attachment,
        PostType::NavMenuItem,
        PostType::WpBlock,
        PostType::WpTemplate,
        PostType::WpTemplatePart,
        PostType::WpNavigation,
        PostType::WpFontFamily,
        PostType::WpFontFace
    )]
    post_type: PostType,
) {
    api_client()
        .post_types()
        .retrieve_with_view_context(&post_type)
        .await
        .assert_response();
}

mod filter {
    use super::*;

    wp_api::generate_sparse_post_type_details_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_post_type_details_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_post_type_details_field_with_view_context_test_cases!();

    #[apply(sparse_post_type_details_field_with_edit_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_post_types_with_edit_context(
        #[values(
            PostType::Post,
            PostType::Page,
            PostType::Attachment,
            PostType::NavMenuItem,
            PostType::WpBlock,
            PostType::WpTemplate,
            PostType::WpTemplatePart,
            PostType::WpNavigation,
            PostType::WpFontFamily,
            PostType::WpFontFace
        )]
        post_type: PostType,
        #[case] fields: &[SparsePostTypeDetailsFieldWithEditContext],
    ) {
        let p = api_client()
            .post_types()
            .filter_retrieve_with_edit_context(&post_type, fields)
            .await
            .assert_response();
        p.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_post_type_details_field_with_embed_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_post_types_with_embed_context(
        #[values(
            PostType::Post,
            PostType::Page,
            PostType::Attachment,
            PostType::NavMenuItem,
            PostType::WpBlock,
            PostType::WpTemplate,
            PostType::WpTemplatePart,
            PostType::WpNavigation,
            PostType::WpFontFamily,
            PostType::WpFontFace
        )]
        post_type: PostType,
        #[case] fields: &[SparsePostTypeDetailsFieldWithEmbedContext],
    ) {
        let p = api_client()
            .post_types()
            .filter_retrieve_with_embed_context(&post_type, fields)
            .await
            .assert_response();
        p.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_post_type_details_field_with_view_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_post_types_with_view_context(
        #[values(
            PostType::Post,
            PostType::Page,
            PostType::Attachment,
            PostType::NavMenuItem,
            PostType::WpBlock,
            PostType::WpTemplate,
            PostType::WpTemplatePart,
            PostType::WpNavigation,
            PostType::WpFontFamily,
            PostType::WpFontFace
        )]
        post_type: PostType,
        #[case] fields: &[SparsePostTypeDetailsFieldWithViewContext],
    ) {
        let p = api_client()
            .post_types()
            .filter_retrieve_with_view_context(&post_type, fields)
            .await
            .assert_response();
        p.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}
