use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::{
    generate,
    media::{
        MediaListParams, SparseMediaFieldWithEditContext, SparseMediaFieldWithEmbedContext,
        SparseMediaFieldWithViewContext,
    },
    JsonValue,
};
use wp_api_integration_tests::{api_client, AssertResponse};

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: MediaListParams) {
    let media_list = api_client()
        .media()
        .list_with_edit_context(&params)
        .await
        .assert_response()
        .data;
    // Temporary checks to see if JsonValue implementation is working
    media_list.into_iter().for_each(|m| {
        match m.media_details {
            JsonValue::Object(hash_map) => {
                //let w = hash_map.get("width").expect("All media in our test site seems to return a width, but this is probably not guaranteed.");
                let w = hash_map.get("width");
                if w.is_some() {
                    let w = w.unwrap();
                    match w {
                        JsonValue::Int(json_number) => {
                            println!("width: {:#?}", json_number);
                        }
                        _ => panic!("Width should be a number"),
                    }
                } else {
                    println!("{:#?}", hash_map);
                }
            }
            _ => panic!("Media details should be a JSON object"),
        }
    });
}

#[template]
#[rstest]
#[case::default(MediaListParams::default())]
#[case::page(generate!(MediaListParams, (page, Some(1))))]
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
}
