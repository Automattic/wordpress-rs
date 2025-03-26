use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{
    SparseField,
    tags::{
        SparseTagFieldWithEditContext, SparseTagFieldWithEmbedContext,
        SparseTagFieldWithViewContext, TagId, TagListParams,
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum TagsRequest {
    #[contextual_paged(url = "/tags", params = &TagListParams, output = Vec<crate::tags::SparseTag>, filter_by = crate::tags::SparseTagField)]
    List,
    #[contextual_get(url = "/tags/<tag_id>", output = crate::tags::SparseTag, filter_by = crate::tags::SparseTagField)]
    Retrieve,
    #[post(url = "/tags", params = &crate::tags::TagCreateParams, output = crate::tags::TagWithEditContext)]
    Create,
    #[delete(url = "/tags/<tag_id>", output = crate::tags::TagDeleteResponse)]
    Delete,
    #[post(url = "/tags/<tag_id>", params = &crate::tags::TagUpdateParams, output = crate::tags::TagWithEditContext)]
    Update,
}

impl DerivedRequest for TagsRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            // The server always returns an error when `force=false`, so a separate `Trash` action
            // is not implemented.
            Self::Delete => vec![("force", true.to_string())],
            _ => vec![],
        }
    }

    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

super::macros::default_sparse_field_implementation_from_field_name!(SparseTagFieldWithEditContext);
super::macros::default_sparse_field_implementation_from_field_name!(SparseTagFieldWithEmbedContext);
super::macros::default_sparse_field_implementation_from_field_name!(SparseTagFieldWithViewContext);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParsedUrl;
    use crate::{
        WpApiParamOrder, generate,
        posts::PostId,
        request::endpoint::tests::{fixture_api_root_url, validate_wp_v2_endpoint},
        tags::{TagId, WpApiParamTagsOrderBy},
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(TagListParams::default(), "")]
    #[case(generate!(TagListParams, (page, Some(2))), "page=2")]
    #[case(generate!(TagListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(TagListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(TagListParams, (exclude, vec![TagId(1), TagId(2)])), "exclude=1%2C2")]
    #[case(generate!(TagListParams, (include, vec![TagId(1), TagId(2)])), "include=1%2C2")]
    #[case(generate!(TagListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(TagListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(TagListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::Id))), "orderby=id")]
    #[case(generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::Include))), "orderby=include")]
    #[case(generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::Name))), "orderby=name")]
    #[case(generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::TermGroup))), "orderby=term_group")]
    #[case(generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::Description))), "orderby=description")]
    #[case(generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::Count))), "orderby=count")]
    #[case(generate!(TagListParams, (hide_empty, Some(true))), "hide_empty=true")]
    #[case(generate!(TagListParams, (post, Some(PostId(3)))), "post=3")]
    #[case(generate!(TagListParams, (slug, vec!["slug_1".to_string(), "slug_2".to_string()])), "slug=slug_1%2Cslug_2")]
    // TODO
    #[case(
        tag_list_params_with_all_fields(),
        EXPECTED_QUERY_PAIRS_FOR_TAG_LIST_PARAMS_WITH_ALL_FIELDS
    )]
    fn list_tags(
        endpoint: TagsRequestEndpoint,
        #[case] params: TagListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/tags?context={}", context)
            } else {
                format!("/tags?context={}&{}", context, expected_additional_params)
            }
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&params),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(TagListParams::default(), &[], "/tags?context=edit&_fields=")]
    #[case(generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::Id))), &[SparseTagFieldWithEditContext::Count], "/tags?context=edit&orderby=id&_fields=count")]
    #[case(tag_list_params_with_all_fields(), ALL_SPARSE_TAG_FIELDS_WITH_EDIT_CONTEXT, &format!("/tags?context=edit&{}&{}", EXPECTED_QUERY_PAIRS_FOR_TAG_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TAG_FIELDS_WITH_EDIT_CONTEXT))]
    fn filter_list_tags_with_edit_context(
        endpoint: TagsRequestEndpoint,
        #[case] params: TagListParams,
        #[case] fields: &[SparseTagFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(TagListParams::default(), &[], "/tags?context=embed&_fields=")]
    #[case(generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::Slug))), &[SparseTagFieldWithEmbedContext::Link], "/tags?context=embed&orderby=slug&_fields=link")]
    #[case(tag_list_params_with_all_fields(), ALL_SPARSE_TAG_FIELDS_WITH_EMBED_CONTEXT, &format!("/tags?context=embed&{}&{}", EXPECTED_QUERY_PAIRS_FOR_TAG_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TAG_FIELDS_WITH_EMBED_CONTEXT))]
    fn filter_list_tags_with_embed_context(
        endpoint: TagsRequestEndpoint,
        #[case] params: TagListParams,
        #[case] fields: &[SparseTagFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_embed_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(TagListParams::default(), &[], "/tags?context=view&_fields=")]
    #[case(generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::Include))), &[SparseTagFieldWithViewContext::Description], "/tags?context=view&orderby=include&_fields=description")]
    #[case(tag_list_params_with_all_fields(), ALL_SPARSE_TAG_FIELDS_WITH_VIEW_CONTEXT, &format!("/tags?context=view&{}&{}", EXPECTED_QUERY_PAIRS_FOR_TAG_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TAG_FIELDS_WITH_VIEW_CONTEXT))]
    fn filter_list_tags_with_view_context(
        endpoint: TagsRequestEndpoint,
        #[case] params: TagListParams,
        #[case] fields: &[SparseTagFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&params, fields),
            expected_path,
        );
    }

    const EXPECTED_QUERY_PAIRS_FOR_TAG_LIST_PARAMS_WITH_ALL_FIELDS: &str = "page=11&per_page=22&search=s_q&exclude=1111%2C1112&include=2111%2C2112&offset=11111&order=desc&orderby=slug&hide_empty=true&post=33333&slug=slug_1%2Cslug_2";
    fn tag_list_params_with_all_fields() -> TagListParams {
        TagListParams {
            page: Some(11),
            per_page: Some(22),
            search: Some("s_q".to_string()),
            exclude: vec![TagId(1111), TagId(1112)],
            include: vec![TagId(2111), TagId(2112)],
            offset: Some(11111),
            order: Some(WpApiParamOrder::Desc),
            orderby: Some(WpApiParamTagsOrderBy::Slug),
            hide_empty: Some(true),
            post: Some(PostId(33333)),
            slug: vec!["slug_1".to_string(), "slug_2".to_string()],
        }
    }

    #[rstest]
    fn retrieve_tag(endpoint: TagsRequestEndpoint) {
        let tag_id = TagId(54);
        let expected_path = |context: &str| format!("/tags/54?context={}", context);
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&tag_id),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&tag_id),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&tag_id),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(&[], "/tags/54?context=view&_fields=")]
    #[case(&[SparseTagFieldWithViewContext::Count], "/tags/54?context=view&_fields=count")]
    #[case(ALL_SPARSE_TAG_FIELDS_WITH_VIEW_CONTEXT, &format!("/tags/54?context=view&{}", EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TAG_FIELDS_WITH_VIEW_CONTEXT))]
    fn filter_retrieve_tag_with_view_context(
        endpoint: TagsRequestEndpoint,
        #[case] fields: &[SparseTagFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(&TagId(54), fields),
            expected_path,
        );
    }

    #[rstest]
    fn create_tag(endpoint: TagsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(), "/tags");
    }

    #[rstest]
    fn delete_tag(endpoint: TagsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.delete(&TagId(54)), "/tags/54?force=true");
    }

    #[rstest]
    fn update_tag(endpoint: TagsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.update(&TagId(54)), "/tags/54");
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TAG_FIELDS_WITH_EDIT_CONTEXT: &str =
        "_fields=id%2Ccount%2Cdescription%2Clink%2Cname%2Cslug%2Ctaxonomy";
    const ALL_SPARSE_TAG_FIELDS_WITH_EDIT_CONTEXT: &[SparseTagFieldWithEditContext; 7] = &[
        SparseTagFieldWithEditContext::Id,
        SparseTagFieldWithEditContext::Count,
        SparseTagFieldWithEditContext::Description,
        SparseTagFieldWithEditContext::Link,
        SparseTagFieldWithEditContext::Name,
        SparseTagFieldWithEditContext::Slug,
        SparseTagFieldWithEditContext::Taxonomy,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TAG_FIELDS_WITH_EMBED_CONTEXT: &str =
        "_fields=id%2Clink%2Cname%2Cslug%2Ctaxonomy";
    const ALL_SPARSE_TAG_FIELDS_WITH_EMBED_CONTEXT: &[SparseTagFieldWithEmbedContext; 5] = &[
        SparseTagFieldWithEmbedContext::Id,
        SparseTagFieldWithEmbedContext::Link,
        SparseTagFieldWithEmbedContext::Name,
        SparseTagFieldWithEmbedContext::Slug,
        SparseTagFieldWithEmbedContext::Taxonomy,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TAG_FIELDS_WITH_VIEW_CONTEXT: &str =
        "_fields=id%2Ccount%2Cdescription%2Clink%2Cname%2Cslug%2Ctaxonomy";
    const ALL_SPARSE_TAG_FIELDS_WITH_VIEW_CONTEXT: &[SparseTagFieldWithViewContext; 7] = &[
        SparseTagFieldWithViewContext::Id,
        SparseTagFieldWithViewContext::Count,
        SparseTagFieldWithViewContext::Description,
        SparseTagFieldWithViewContext::Link,
        SparseTagFieldWithViewContext::Name,
        SparseTagFieldWithViewContext::Slug,
        SparseTagFieldWithViewContext::Taxonomy,
    ];

    #[fixture]
    fn endpoint(fixture_api_root_url: Arc<ParsedUrl>) -> TagsRequestEndpoint {
        TagsRequestEndpoint::new(fixture_api_root_url)
    }
}
