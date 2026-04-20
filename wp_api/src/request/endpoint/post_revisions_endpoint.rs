use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{
    post_revisions::{AnyPostRevisionListParams, PostRevisionId},
    posts::PostId,
    request::endpoint::posts_endpoint::PostEndpointType,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum PostRevisionsRequest {
    #[contextual_paged(url = "/<post_endpoint_type>/<post_id>/revisions", params = &AnyPostRevisionListParams, output = Vec<crate::post_revisions::SparseAnyPostRevision>, filter_by = crate::post_revisions::SparseAnyPostRevisionField)]
    List,
    #[contextual_get(url = "/<post_endpoint_type>/<post_id>/revisions/<post_revision_id>", output = crate::post_revisions::SparseAnyPostRevision, filter_by = crate::post_revisions::SparseAnyPostRevisionField)]
    Retrieve,
    #[delete(url = "/<post_endpoint_type>/<post_id>/revisions/<post_revision_id>", output = crate::post_revisions::AnyPostRevisionDeleteResponse)]
    Delete,
}

impl DerivedRequest for PostRevisionsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }

    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            PostRevisionsRequest::Delete => vec![("force", "true".to_string())],
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::post_revisions::{
        PostRevisionId, SparseAnyPostRevisionFieldWithEditContext, WpApiParamPostRevisionsOrderBy,
    };
    use crate::request::endpoint::ApiUrlResolver;
    use crate::{
        WpApiParamOrder, generate,
        posts::PostId,
        request::endpoint::tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(AnyPostRevisionListParams::default(), "")]
    #[case(generate!(AnyPostRevisionListParams, (page, Some(2))), "page=2")]
    #[case(generate!(AnyPostRevisionListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(AnyPostRevisionListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(AnyPostRevisionListParams, (exclude, vec![PostRevisionId(1), PostRevisionId(2)])), "exclude=1%2C2")]
    #[case(generate!(AnyPostRevisionListParams, (include, vec![PostRevisionId(1), PostRevisionId(2)])), "include=1%2C2")]
    #[case(generate!(AnyPostRevisionListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(AnyPostRevisionListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(AnyPostRevisionListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Date))), "orderby=date")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Id))), "orderby=id")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Include))), "orderby=include")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Relevance))), "orderby=relevance")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Title))), "orderby=title")]
    #[case(
        post_revision_list_params_with_all_fields(),
        &expected_query_pairs_for_post_revision_list_params_with_all_fields()
    )]
    fn list_posts(
        endpoint: PostRevisionsRequestEndpoint,
        #[case] params: AnyPostRevisionListParams,
        #[case] expected_additional_params: &str,
    ) {
        let post_id = PostId(777);
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/posts/{post_id}/revisions?context={context}")
            } else {
                format!("/posts/{post_id}/revisions?context={context}&{expected_additional_params}")
            }
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&PostEndpointType::Posts, &post_id, &params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&PostEndpointType::Posts, &post_id, &params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&PostEndpointType::Posts, &post_id, &params),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(AnyPostRevisionListParams::default(), &[], "/posts/777/revisions?context=edit&_fields=")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Id))), &[SparseAnyPostRevisionFieldWithEditContext::Date], "/posts/777/revisions?context=edit&orderby=id&_fields=date")]
    #[case(post_revision_list_params_with_all_fields(), ALL_SPARSE_POST_REVISION_FIELDS_WITH_EDIT_CONTEXT, &format!("/posts/777/revisions?context=edit&{}&{}", expected_query_pairs_for_post_revision_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_REVISION_FIELDS_WITH_EDIT_CONTEXT))]
    fn filter_list_post_revision_with_edit_context(
        endpoint: PostRevisionsRequestEndpoint,
        #[case] params: AnyPostRevisionListParams,
        #[case] fields: &[SparseAnyPostRevisionFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(
                &PostEndpointType::Posts,
                &PostId(777),
                &params,
                fields,
            ),
            expected_path,
        );
    }

    fn expected_query_pairs_for_post_revision_list_params_with_all_fields() -> String {
        "page=2&per_page=2&search=foo&exclude=1%2C2&include=1%2C2&offset=2&order=asc&orderby=id"
            .to_string()
    }

    fn post_revision_list_params_with_all_fields() -> AnyPostRevisionListParams {
        AnyPostRevisionListParams {
            page: Some(2),
            per_page: Some(2),
            search: Some("foo".to_string()),
            exclude: vec![PostRevisionId(1), PostRevisionId(2)],
            include: vec![PostRevisionId(1), PostRevisionId(2)],
            offset: Some(2),
            order: Some(WpApiParamOrder::Asc),
            orderby: Some(WpApiParamPostRevisionsOrderBy::Id),
        }
    }

    #[rstest]
    fn retrieve_post_revision(endpoint: PostRevisionsRequestEndpoint) {
        let post_id = PostId(777);
        let revision_id = PostRevisionId(888);
        let expected_path =
            |context: &str| format!("/posts/{post_id}/revisions/{revision_id}?context={context}");
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&PostEndpointType::Posts, &post_id, &revision_id),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&PostEndpointType::Posts, &post_id, &revision_id),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&PostEndpointType::Posts, &post_id, &revision_id),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(&[], "/posts/777/revisions/888?context=edit&_fields=")]
    #[case(&[SparseAnyPostRevisionFieldWithEditContext::Date], "/posts/777/revisions/888?context=edit&_fields=date")]
    fn filter_retrieve_post_revision_with_edit_context(
        endpoint: PostRevisionsRequestEndpoint,
        #[case] fields: &[SparseAnyPostRevisionFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_edit_context(
                &PostEndpointType::Posts,
                &PostId(777),
                &PostRevisionId(888),
                fields,
            ),
            expected_path,
        );
    }

    #[rstest]
    fn delete_post_revision(endpoint: PostRevisionsRequestEndpoint) {
        let post_id = PostId(777);
        let revision_id = PostRevisionId(888);
        validate_wp_v2_endpoint(
            endpoint.delete(&PostEndpointType::Posts, &post_id, &revision_id),
            &format!("/posts/{post_id}/revisions/{revision_id}?force=true"),
        );
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_REVISION_FIELDS_WITH_EDIT_CONTEXT: &str = "_fields=id%2Cauthor%2Cdate%2Cdate_gmt%2Cmodified%2Cmodified_gmt%2Cparent%2Cslug%2Cguid%2Ctitle%2Ccontent%2Cexcerpt%2Cmeta";
    const ALL_SPARSE_POST_REVISION_FIELDS_WITH_EDIT_CONTEXT: &[SparseAnyPostRevisionFieldWithEditContext;
         13] = &[
        SparseAnyPostRevisionFieldWithEditContext::Id,
        SparseAnyPostRevisionFieldWithEditContext::Author,
        SparseAnyPostRevisionFieldWithEditContext::Date,
        SparseAnyPostRevisionFieldWithEditContext::DateGmt,
        SparseAnyPostRevisionFieldWithEditContext::Modified,
        SparseAnyPostRevisionFieldWithEditContext::ModifiedGmt,
        SparseAnyPostRevisionFieldWithEditContext::Parent,
        SparseAnyPostRevisionFieldWithEditContext::Slug,
        SparseAnyPostRevisionFieldWithEditContext::Guid,
        SparseAnyPostRevisionFieldWithEditContext::Title,
        SparseAnyPostRevisionFieldWithEditContext::Content,
        SparseAnyPostRevisionFieldWithEditContext::Excerpt,
        SparseAnyPostRevisionFieldWithEditContext::Meta,
    ];

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> PostRevisionsRequestEndpoint {
        PostRevisionsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
