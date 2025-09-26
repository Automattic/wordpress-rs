use wp_api::post_statuses::{
    SparsePostStatusFieldWithEditContext, SparsePostStatusFieldWithEmbedContext,
    SparsePostStatusFieldWithViewContext,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    api_client()
        .post_statuses()
        .list_with_edit_context()
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .post_statuses()
        .list_with_embed_context()
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .post_statuses()
        .list_with_view_context()
        .await
        .assert_response();
}

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
async fn retrieve_with_edit_context(#[case] status_slug: &str) {
    api_client()
        .post_statuses()
        .retrieve_with_edit_context(&status_slug.into())
        .await
        .assert_response();
}

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
async fn retrieve_with_embed_context(#[case] status_slug: &str) {
    api_client()
        .post_statuses()
        .retrieve_with_embed_context(&status_slug.into())
        .await
        .assert_response();
}

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
async fn retrieve_with_view_context(#[case] status_slug: &str) {
    api_client()
        .post_statuses()
        .retrieve_with_view_context(&status_slug.into())
        .await
        .assert_response();
}

#[template]
#[rstest]
#[case::publish("publish")]
#[case::future("future")]
#[case::draft("draft")]
#[case::pending("pending")]
#[case::private("private")]
#[case::trash("trash")]
pub fn retrieve_cases(#[case] status_slug: &str) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_post_status_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_post_status_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_post_status_field_with_view_context_test_cases!();

    #[apply(sparse_post_status_field_with_edit_context_test_cases)]
    #[case(&[SparsePostStatusFieldWithEditContext::Name, SparsePostStatusFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_post_statuses_with_edit_context(
        #[case] fields: &[SparsePostStatusFieldWithEditContext],
    ) {
        let response = api_client()
            .post_statuses()
            .filter_list_with_edit_context(fields)
            .await
            .assert_response()
            .data;

        if let Some(post_statuses) = &response.post_statuses {
            post_statuses.values().for_each(|status| {
                status.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
        }
    }

    #[apply(sparse_post_status_field_with_edit_context_test_cases)]
    #[case(&[SparsePostStatusFieldWithEditContext::Name, SparsePostStatusFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_post_status_with_edit_context(
        #[case] fields: &[SparsePostStatusFieldWithEditContext],
    ) {
        let status = api_client()
            .post_statuses()
            .filter_retrieve_with_edit_context(&"publish".into(), fields)
            .await
            .assert_response()
            .data;

        status.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_post_status_field_with_embed_context_test_cases)]
    #[case(&[SparsePostStatusFieldWithEmbedContext::Name, SparsePostStatusFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_post_statuses_with_embed_context(
        #[case] fields: &[SparsePostStatusFieldWithEmbedContext],
    ) {
        let response = api_client()
            .post_statuses()
            .filter_list_with_embed_context(fields)
            .await
            .assert_response()
            .data;

        if let Some(post_statuses) = &response.post_statuses {
            post_statuses.values().for_each(|status| {
                status.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
        }
    }

    #[apply(sparse_post_status_field_with_embed_context_test_cases)]
    #[case(&[SparsePostStatusFieldWithEmbedContext::Name, SparsePostStatusFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_post_status_with_embed_context(
        #[case] fields: &[SparsePostStatusFieldWithEmbedContext],
    ) {
        let status = api_client()
            .post_statuses()
            .filter_retrieve_with_embed_context(&"publish".into(), fields)
            .await
            .assert_response()
            .data;

        status.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_post_status_field_with_view_context_test_cases)]
    #[case(&[SparsePostStatusFieldWithViewContext::Name, SparsePostStatusFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_post_statuses_with_view_context(
        #[case] fields: &[SparsePostStatusFieldWithViewContext],
    ) {
        let response = api_client()
            .post_statuses()
            .filter_list_with_view_context(fields)
            .await
            .assert_response()
            .data;

        if let Some(post_statuses) = &response.post_statuses {
            post_statuses.values().for_each(|status| {
                status.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
        }
    }

    #[apply(sparse_post_status_field_with_view_context_test_cases)]
    #[case(&[SparsePostStatusFieldWithViewContext::Name, SparsePostStatusFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_post_status_with_view_context(
        #[case] fields: &[SparsePostStatusFieldWithViewContext],
    ) {
        let status = api_client()
            .post_statuses()
            .filter_retrieve_with_view_context(&"publish".into(), fields)
            .await
            .assert_response()
            .data;

        status.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}
