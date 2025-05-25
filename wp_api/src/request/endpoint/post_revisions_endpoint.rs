use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{
    SparseField,
    post_revisions::{
        PostRevisionListParams, SparsePostRevisionFieldWithEditContext,
        SparsePostRevisionFieldWithEmbedContext, SparsePostRevisionFieldWithViewContext,
    },
    posts::PostId,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum PostRevisionsRequest {
    #[contextual_paged(url = "/posts/<post_id>/revisions", params = &PostRevisionListParams, output = Vec<crate::post_revisions::SparsePostRevision>, filter_by = crate::post_revisions::SparsePostRevisionField)]
    List,
}

impl DerivedRequest for PostRevisionsRequest {
    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

super::macros::default_sparse_field_implementation_from_field_name!(
    SparsePostRevisionFieldWithEditContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparsePostRevisionFieldWithEmbedContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparsePostRevisionFieldWithViewContext
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::post_revisions::{PostRevisionId, WpApiParamPostRevisionsOrderBy};
    use crate::request::endpoint::ApiUrlResolver;
    use crate::{
        WpApiParamOrder, generate,
        posts::PostId,
        request::endpoint::tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(PostRevisionListParams::default(), "")]
    #[case(generate!(PostRevisionListParams, (page, Some(2))), "page=2")]
    #[case(generate!(PostRevisionListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(PostRevisionListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(PostRevisionListParams, (exclude, vec![PostRevisionId(1), PostRevisionId(2)])), "exclude=1%2C2")]
    #[case(generate!(PostRevisionListParams, (include, vec![PostRevisionId(1), PostRevisionId(2)])), "include=1%2C2")]
    #[case(generate!(PostRevisionListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(PostRevisionListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(PostRevisionListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(PostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Date))), "orderby=date")]
    #[case(generate!(PostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Id))), "orderby=id")]
    #[case(generate!(PostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Include))), "orderby=include")]
    #[case(generate!(PostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(PostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Relevance))), "orderby=relevance")]
    #[case(generate!(PostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(PostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Title))), "orderby=title")]
    #[case(
        post_revision_list_params_with_all_fields(),
        &expected_query_pairs_for_post_revision_list_params_with_all_fields()
    )]
    fn list_posts(
        endpoint: PostRevisionsRequestEndpoint,
        #[case] params: PostRevisionListParams,
        #[case] expected_additional_params: &str,
    ) {
        let post_id = PostId(777);
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/posts/{post_id}/revisions?context={}", context)
            } else {
                format!(
                    "/posts/{post_id}/revisions?context={}&{}",
                    context, expected_additional_params
                )
            }
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&post_id, &params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&post_id, &params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&post_id, &params),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(PostRevisionListParams::default(), &[], "/posts/777/revisions?context=edit&_fields=")]
    #[case(generate!(PostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Id))), &[SparsePostRevisionFieldWithEditContext::Date], "/posts/777/revisions?context=edit&orderby=id&_fields=date")]
    #[case(post_revision_list_params_with_all_fields(), ALL_SPARSE_POST_REVISION_FIELDS_WITH_EDIT_CONTEXT, &format!("/posts/777/revisions?context=edit&{}&{}", expected_query_pairs_for_post_revision_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_REVISION_FIELDS_WITH_EDIT_CONTEXT))]
    fn filter_list_post_revision_with_edit_context(
        endpoint: PostRevisionsRequestEndpoint,
        #[case] params: PostRevisionListParams,
        #[case] fields: &[SparsePostRevisionFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&PostId(777), &params, fields),
            expected_path,
        );
    }

    fn expected_query_pairs_for_post_revision_list_params_with_all_fields() -> String {
        "page=2&per_page=2&search=foo&exclude=1%2C2&include=1%2C2&offset=2&order=asc&orderby=id"
            .to_string()
    }

    fn post_revision_list_params_with_all_fields() -> PostRevisionListParams {
        PostRevisionListParams {
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

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_REVISION_FIELDS_WITH_EDIT_CONTEXT: &str = "_fields=id%2Cauthor%2Cdate%2Cdate_gmt%2Cmodified%2Cmodified_gmt%2Cparent%2Cslug%2Cguid%2Ctitle%2Ccontent%2Cexcerpt%2Cmeta";
    const ALL_SPARSE_POST_REVISION_FIELDS_WITH_EDIT_CONTEXT: &[SparsePostRevisionFieldWithEditContext;
         13] = &[
        SparsePostRevisionFieldWithEditContext::Id,
        SparsePostRevisionFieldWithEditContext::Author,
        SparsePostRevisionFieldWithEditContext::Date,
        SparsePostRevisionFieldWithEditContext::DateGmt,
        SparsePostRevisionFieldWithEditContext::Modified,
        SparsePostRevisionFieldWithEditContext::ModifiedGmt,
        SparsePostRevisionFieldWithEditContext::Parent,
        SparsePostRevisionFieldWithEditContext::Slug,
        SparsePostRevisionFieldWithEditContext::Guid,
        SparsePostRevisionFieldWithEditContext::Title,
        SparsePostRevisionFieldWithEditContext::Content,
        SparsePostRevisionFieldWithEditContext::Excerpt,
        SparsePostRevisionFieldWithEditContext::Meta,
    ];

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> PostRevisionsRequestEndpoint {
        PostRevisionsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
