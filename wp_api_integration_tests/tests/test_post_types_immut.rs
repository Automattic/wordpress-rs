use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::post_types::{
    PostType, PostTypeSupports, SparsePostTypeDetails, SparsePostTypeDetailsField,
    SparsePostTypeDetailsFieldWithEditContext,
};
use wp_api::WpContext;

use crate::integration_test_common::{api_client, AssertResponse};

pub mod integration_test_common;

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

#[apply(filter_fields_cases)]
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
    #[case] fields: &[SparsePostTypeDetailsField],
) {
    let p = api_client()
        .post_types()
        .filter_retrieve(&post_type, WpContext::Edit, fields)
        .await
        .assert_response();
    validate_sparse_post_type_fields_with_edit_context(&p, fields);
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
    // To be clear, if we can't parse it as expected, we'd get an error back and the previous
    // assertion would fail, but having some defensive validation may help guard against
    // future changes.
    //
    // It's entirely possible that we might have more test sites in the future and some of their
    // post types might not support a `Title` in which case it's perfectly fine to completely
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

fn validate_sparse_post_type_fields_with_edit_context(
    post_type_details: &SparsePostTypeDetails,
    fields: &[SparsePostTypeDetailsField],
) {
    let field_included = |field| {
        // If "fields" is empty the server will return all fields
        fields.is_empty() || fields.contains(&field)
    };
    assert_eq!(
        post_type_details.capabilities.is_some(),
        field_included(SparsePostTypeDetailsField::WithEditContext(
            SparsePostTypeDetailsFieldWithEditContext::Capabilities
        ))
    );
    assert_eq!(
        post_type_details.description.is_some(),
        field_included(SparsePostTypeDetailsField::WithEditContext(
            SparsePostTypeDetailsFieldWithEditContext::Description
        ))
    );
    assert_eq!(
        post_type_details.hierarchical.is_some(),
        field_included(SparsePostTypeDetailsField::WithEditContext(
            SparsePostTypeDetailsFieldWithEditContext::Hierarchical
        ))
    );
    assert_eq!(
        post_type_details.viewable.is_some(),
        field_included(SparsePostTypeDetailsField::WithEditContext(
            SparsePostTypeDetailsFieldWithEditContext::Viewable
        ))
    );
    assert_eq!(
        post_type_details.labels.is_some(),
        field_included(SparsePostTypeDetailsField::WithEditContext(
            SparsePostTypeDetailsFieldWithEditContext::Labels
        ))
    );
    assert_eq!(
        post_type_details.name.is_some(),
        field_included(SparsePostTypeDetailsField::WithEditContext(
            SparsePostTypeDetailsFieldWithEditContext::Name
        ))
    );
    assert_eq!(
        post_type_details.slug.is_some(),
        field_included(SparsePostTypeDetailsField::WithEditContext(
            SparsePostTypeDetailsFieldWithEditContext::Slug
        ))
    );
    assert_eq!(
        post_type_details.supports.is_some(),
        field_included(SparsePostTypeDetailsField::WithEditContext(
            SparsePostTypeDetailsFieldWithEditContext::Supports
        ))
    );
    assert_eq!(
        post_type_details.has_archive.is_some(),
        field_included(SparsePostTypeDetailsField::WithEditContext(
            SparsePostTypeDetailsFieldWithEditContext::HasArchive
        ))
    );
    assert_eq!(
        post_type_details.taxonomies.is_some(),
        field_included(SparsePostTypeDetailsField::WithEditContext(
            SparsePostTypeDetailsFieldWithEditContext::Taxonomies
        ))
    );
    assert_eq!(
        post_type_details.rest_base.is_some(),
        field_included(SparsePostTypeDetailsField::WithEditContext(
            SparsePostTypeDetailsFieldWithEditContext::RestBase
        ))
    );
    assert_eq!(
        post_type_details.rest_namespace.is_some(),
        field_included(SparsePostTypeDetailsField::WithEditContext(
            SparsePostTypeDetailsFieldWithEditContext::RestNamespace
        ))
    );
    assert_eq!(
        post_type_details.visibility.is_some(),
        field_included(SparsePostTypeDetailsField::WithEditContext(
            SparsePostTypeDetailsFieldWithEditContext::Visibility
        ))
    );
    // Since the post_type_details.icon can always be null, don't validate it
}

#[template]
#[rstest]
#[case(&[])]
#[case(&[
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::Capabilities)
])]
#[case(&[
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::Description),
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::Hierarchical)
])]
#[case(&[
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::Viewable),
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::Labels),
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::Name),
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::Slug),
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::Supports),
])]
#[case(&[
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::HasArchive),
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::Taxonomies),
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::RestBase),
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::RestNamespace),
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::Visibility),
    SparsePostTypeDetailsField::WithEditContext(SparsePostTypeDetailsFieldWithEditContext::Icon),
])]
fn filter_fields_cases(#[case] fields: &[SparsePostTypeDetailsField]) {}
