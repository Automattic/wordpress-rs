use wp_derive_request_builder::WpDerivedRequest;

use crate::post_types::{PostType, SparsePostTypeDetailsField};

#[derive(WpDerivedRequest)]
#[Namespace("/wp/v2")]
#[SparseField(SparsePostTypeDetailsField)]
enum PostTypesRequest {
    #[contextual_get(url = "/types", output = crate::post_types::SparseListPostTypesResponse)]
    List,
    #[contextual_get(url = "/types/<post_type>", output = crate::post_types::SparsePostTypeDetails)]
    Retrieve,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::endpoint::{
        tests::{fixture_api_base_url, validate_wp_v2_endpoint},
        ApiBaseUrl,
    };
    use crate::WpContext;
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_post_types(endpoint: PostTypesRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.list_with_edit_context(), "/types?context=edit");
        validate_wp_v2_endpoint(endpoint.list_with_embed_context(), "/types?context=embed");
        validate_wp_v2_endpoint(endpoint.list_with_view_context(), "/types?context=view");
    }

    #[rstest]
    #[case(WpContext::Edit, &[SparsePostTypeDetailsField::Capabilities], "/types?context=edit&_fields=capabilities")]
    #[case(WpContext::Embed, &[SparsePostTypeDetailsField::Description, SparsePostTypeDetailsField::Hierarchical], "/types?context=embed&_fields=description%2Chierarchical")]
    #[case(
        WpContext::View,
        &[
            SparsePostTypeDetailsField::Viewable,
            SparsePostTypeDetailsField::Labels,
            SparsePostTypeDetailsField::Name,
            SparsePostTypeDetailsField::Slug,
            SparsePostTypeDetailsField::Supports,
        ],
        "/types?context=view&_fields=viewable%2Clabels%2Cname%2Cslug%2Csupports"
    )]
    fn filter_list_post_types(
        endpoint: PostTypesRequestEndpoint,
        #[case] context: WpContext,
        #[case] fields: &[SparsePostTypeDetailsField],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(endpoint.filter_list(context, fields), expected_path);
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
    #[case(PostType::Post, WpContext::Edit, &[SparsePostTypeDetailsField::Capabilities], "/types/post?context=edit&_fields=capabilities")]
    #[case(
        PostType::WpBlock,
        WpContext::Embed,
        &[
            SparsePostTypeDetailsField::HasArchive,
            SparsePostTypeDetailsField::Taxonomies,
            SparsePostTypeDetailsField::RestBase,
            SparsePostTypeDetailsField::RestNamespace,
            SparsePostTypeDetailsField::Visibility,
            SparsePostTypeDetailsField::Icon,
        ],
        "/types/wp_block?context=embed&_fields=has_archive%2Ctaxonomies%2Crest_base%2Crest_namespace%2Cvisibility%2Cicon"
    )]
    fn filter_retrieve_post_type(
        endpoint: PostTypesRequestEndpoint,
        #[case] post_type: PostType,
        #[case] context: WpContext,
        #[case] fields: &[SparsePostTypeDetailsField],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve(&post_type, context, fields),
            expected_path,
        );
    }

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> PostTypesRequestEndpoint {
        PostTypesRequestEndpoint::new(fixture_api_base_url)
    }
}
