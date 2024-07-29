use super::{DerivedRequest, Namespace};
use crate::post_types::{
    PostType, SparsePostTypeDetailsFieldWithEditContext,
    SparsePostTypeDetailsFieldWithEmbedContext, SparsePostTypeDetailsFieldWithViewContext,
};
use crate::SparseField;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
#[Namespace("/wp/v2")]
#[SparseField(SparsePostTypeDetailsField)]
enum PostTypesRequest {
    #[contextual_get(url = "/types", output = crate::post_types::SparsePostTypesResponse)]
    List,
    #[contextual_get(url = "/types/<post_type>", output = crate::post_types::SparsePostTypeDetails, filter_by = crate::post_types::SparsePostTypeDetailsField)]
    Retrieve,
}

impl DerivedRequest for PostTypesRequest {
    fn namespace() -> Namespace {
        Namespace::WpV2
    }
}

super::macros::default_sparse_field_implementation_from_field_name!(
    SparsePostTypeDetailsFieldWithEditContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparsePostTypeDetailsFieldWithEmbedContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparsePostTypeDetailsFieldWithViewContext
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::endpoint::{
        tests::{fixture_api_base_url, validate_wp_v2_endpoint},
        ApiBaseUrl,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_post_types(endpoint: PostTypesRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.list_with_edit_context(), "/types?context=edit");
        validate_wp_v2_endpoint(endpoint.list_with_embed_context(), "/types?context=embed");
        validate_wp_v2_endpoint(endpoint.list_with_view_context(), "/types?context=view");
    }

    #[rstest]
    #[case(PostType::Post, "/types/post")]
    #[case(PostType::Page, "/types/page")]
    #[case(PostType::Attachment, "/types/attachment")]
    #[case(PostType::NavMenuItem, "/types/nav_menu_item")]
    #[case(PostType::WpBlock, "/types/wp_block")]
    #[case(PostType::WpTemplate, "/types/wp_template")]
    #[case(PostType::WpTemplatePart, "/types/wp_template_part")]
    #[case(PostType::WpNavigation, "/types/wp_navigation")]
    #[case(PostType::WpFontFamily, "/types/wp_font_family")]
    #[case(PostType::WpFontFace, "/types/wp_font_face")]
    fn retrieve_post_type(
        endpoint: PostTypesRequestEndpoint,
        #[case] post_type: PostType,
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&post_type),
            format!("{}?context=edit", expected_path).as_str(),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&post_type),
            format!("{}?context=embed", expected_path).as_str(),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&post_type),
            format!("{}?context=view", expected_path).as_str(),
        );
    }

    #[rstest]
    #[case(PostType::Post, &[SparsePostTypeDetailsFieldWithEmbedContext::Name], "/types/post?context=embed&_fields=name")]
    #[case(
        PostType::WpBlock,
        &[
            SparsePostTypeDetailsFieldWithEmbedContext::RestBase,
            SparsePostTypeDetailsFieldWithEmbedContext::RestNamespace,
            SparsePostTypeDetailsFieldWithEmbedContext::Icon,
        ],
        "/types/wp_block?context=embed&_fields=rest_base%2Crest_namespace%2Cicon"
    )]
    fn filter_retrieve_post_type_with_embed_context(
        endpoint: PostTypesRequestEndpoint,
        #[case] post_type: PostType,
        #[case] fields: &[SparsePostTypeDetailsFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_embed_context(&post_type, fields),
            expected_path,
        );
    }

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> PostTypesRequestEndpoint {
        PostTypesRequestEndpoint::new(fixture_api_base_url)
    }
}
