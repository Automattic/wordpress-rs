use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{
    block_revisions::{BlockRevisionId, BlockRevisionListParams},
    blocks::BlockId,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum BlockRevisionsRequest {
    #[contextual_paged(url = "/blocks/<block_id>/revisions", params = &BlockRevisionListParams, output = Vec<crate::block_revisions::SparseBlockRevision>, filter_by = crate::block_revisions::SparseBlockRevisionField)]
    List,
    #[contextual_get(url = "/blocks/<block_id>/revisions/<block_revision_id>", output = crate::block_revisions::SparseBlockRevision, filter_by = crate::block_revisions::SparseBlockRevisionField)]
    Retrieve,
    #[delete(url = "/blocks/<block_id>/revisions/<block_revision_id>", output = crate::block_revisions::BlockRevisionDeleteResponse)]
    Delete,
}

impl DerivedRequest for BlockRevisionsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }

    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            BlockRevisionsRequest::Delete => vec![("force", "true".to_string())],
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        WpApiParamOrder,
        block_revisions::{
            BlockRevisionId, BlockRevisionListParams, WpApiParamBlockRevisionsOrderBy,
        },
        blocks::BlockId,
        generate,
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(BlockRevisionListParams::default(), "")]
    #[case(generate!(BlockRevisionListParams, (page, Some(2))), "page=2")]
    #[case(generate!(BlockRevisionListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(BlockRevisionListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(BlockRevisionListParams, (exclude, vec![BlockRevisionId(1), BlockRevisionId(2)])), "exclude=1%2C2")]
    #[case(generate!(BlockRevisionListParams, (include, vec![BlockRevisionId(1), BlockRevisionId(2)])), "include=1%2C2")]
    #[case(generate!(BlockRevisionListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(BlockRevisionListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(BlockRevisionListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(BlockRevisionListParams, (orderby, Some(WpApiParamBlockRevisionsOrderBy::Date))), "orderby=date")]
    #[case(generate!(BlockRevisionListParams, (orderby, Some(WpApiParamBlockRevisionsOrderBy::Id))), "orderby=id")]
    #[case(generate!(BlockRevisionListParams, (orderby, Some(WpApiParamBlockRevisionsOrderBy::Include))), "orderby=include")]
    #[case(generate!(BlockRevisionListParams, (orderby, Some(WpApiParamBlockRevisionsOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(BlockRevisionListParams, (orderby, Some(WpApiParamBlockRevisionsOrderBy::Relevance))), "orderby=relevance")]
    #[case(generate!(BlockRevisionListParams, (orderby, Some(WpApiParamBlockRevisionsOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(BlockRevisionListParams, (orderby, Some(WpApiParamBlockRevisionsOrderBy::Title))), "orderby=title")]
    fn list_block_revisions(
        endpoint: BlockRevisionsRequestEndpoint,
        #[case] params: BlockRevisionListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/blocks/42/revisions?context={context}")
            } else {
                format!("/blocks/42/revisions?context={context}&{expected_additional_params}")
            }
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&BlockId(42), &params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&BlockId(42), &params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&BlockId(42), &params),
            &expected_path("view"),
        );
    }

    #[rstest]
    fn retrieve_block_revision(endpoint: BlockRevisionsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&BlockId(42), &BlockRevisionId(99)),
            "/blocks/42/revisions/99?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&BlockId(42), &BlockRevisionId(99)),
            "/blocks/42/revisions/99?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&BlockId(42), &BlockRevisionId(99)),
            "/blocks/42/revisions/99?context=view",
        );
    }

    #[rstest]
    fn delete_block_revision(endpoint: BlockRevisionsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.delete(&BlockId(42), &BlockRevisionId(99)),
            "/blocks/42/revisions/99?force=true",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> BlockRevisionsRequestEndpoint {
        BlockRevisionsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
