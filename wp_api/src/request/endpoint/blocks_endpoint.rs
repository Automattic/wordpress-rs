use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::blocks::{BlockId, BlockListParams, BlockRetrieveParams};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum BlocksRequest {
    #[contextual_paged(url = "/blocks", params = &BlockListParams, output = Vec<crate::blocks::SparseBlock>, filter_by = crate::blocks::SparseBlockField)]
    List,
    #[contextual_get(url = "/blocks/<block_id>", params = &BlockRetrieveParams, output = crate::blocks::SparseBlock, filter_by = crate::blocks::SparseBlockField)]
    Retrieve,
    #[post(url = "/blocks", params = &crate::blocks::BlockCreateParams, output = crate::blocks::BlockWithEditContext)]
    Create,
    #[delete(url = "/blocks/<block_id>", output = crate::blocks::BlockDeleteResponse)]
    Delete,
    #[delete(url = "/blocks/<block_id>", output = crate::blocks::BlockWithEditContext)]
    Trash,
    #[post(url = "/blocks/<block_id>", params = &crate::blocks::BlockUpdateParams, output = crate::blocks::BlockWithEditContext)]
    Update,
}

impl DerivedRequest for BlocksRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            BlocksRequest::Delete => vec![("force", true.to_string())],
            BlocksRequest::Trash => vec![("force", false.to_string())],
            _ => vec![],
        }
    }

    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        WpApiParamOrder,
        blocks::{
            BlockId, BlockListParams, BlockRetrieveParams, BlockStatus, WpApiParamBlocksOrderBy,
        },
        generate,
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
        unit_test_common::{
            unit_test_example_date_string_as_option, unit_test_example_date_string_as_query_value,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(BlockListParams::default(), "")]
    #[case(generate!(BlockListParams, (page, Some(2))), "page=2")]
    #[case(generate!(BlockListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(BlockListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(BlockListParams, (after, unit_test_example_date_string_as_option())), &unit_test_example_date_string_as_query_value("after"))]
    #[case(generate!(BlockListParams, (modified_after, unit_test_example_date_string_as_option())), &unit_test_example_date_string_as_query_value("modified_after"))]
    #[case(generate!(BlockListParams, (before, unit_test_example_date_string_as_option())), &unit_test_example_date_string_as_query_value("before"))]
    #[case(generate!(BlockListParams, (modified_before, unit_test_example_date_string_as_option())), &unit_test_example_date_string_as_query_value("modified_before"))]
    #[case(generate!(BlockListParams, (exclude, vec![BlockId(1), BlockId(2)])), "exclude=1%2C2")]
    #[case(generate!(BlockListParams, (include, vec![BlockId(1), BlockId(2)])), "include=1%2C2")]
    #[case(generate!(BlockListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(BlockListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(BlockListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(BlockListParams, (order_by, Some(WpApiParamBlocksOrderBy::Author))), "orderby=author")]
    #[case(generate!(BlockListParams, (order_by, Some(WpApiParamBlocksOrderBy::Date))), "orderby=date")]
    #[case(generate!(BlockListParams, (order_by, Some(WpApiParamBlocksOrderBy::Id))), "orderby=id")]
    #[case(generate!(BlockListParams, (order_by, Some(WpApiParamBlocksOrderBy::Include))), "orderby=include")]
    #[case(generate!(BlockListParams, (order_by, Some(WpApiParamBlocksOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(BlockListParams, (order_by, Some(WpApiParamBlocksOrderBy::Modified))), "orderby=modified")]
    #[case(generate!(BlockListParams, (order_by, Some(WpApiParamBlocksOrderBy::Parent))), "orderby=parent")]
    #[case(generate!(BlockListParams, (order_by, Some(WpApiParamBlocksOrderBy::Relevance))), "orderby=relevance")]
    #[case(generate!(BlockListParams, (order_by, Some(WpApiParamBlocksOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(BlockListParams, (order_by, Some(WpApiParamBlocksOrderBy::Title))), "orderby=title")]
    #[case(generate!(BlockListParams, (search_columns, vec!["post_content".to_string(), "post_excerpt".to_string()])), "search_columns=post_content%2Cpost_excerpt")]
    #[case(generate!(BlockListParams, (slug, vec!["foo".to_string(), "bar".to_string()])), "slug=foo%2Cbar")]
    #[case(generate!(BlockListParams, (status, vec![BlockStatus::Publish])), "status=publish")]
    #[case(generate!(BlockListParams, (status, vec![BlockStatus::Draft, BlockStatus::Publish])), "status=draft%2Cpublish")]
    fn list_blocks(
        endpoint: BlocksRequestEndpoint,
        #[case] params: BlockListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/blocks?context={context}")
            } else {
                format!("/blocks?context={context}&{expected_additional_params}")
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
    #[case(BlockRetrieveParams::default(), "")]
    #[case(generate!(BlockRetrieveParams, (password, Some("secret".to_string()))), "password=secret")]
    fn retrieve_block(
        endpoint: BlocksRequestEndpoint,
        #[case] params: BlockRetrieveParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/blocks/54?context={context}")
            } else {
                format!("/blocks/54?context={context}&{expected_additional_params}")
            }
        };
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&BlockId(54), &params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&BlockId(54), &params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&BlockId(54), &params),
            &expected_path("view"),
        );
    }

    #[rstest]
    fn create_block(endpoint: BlocksRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(), "/blocks");
    }

    #[rstest]
    fn delete_block(endpoint: BlocksRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.delete(&BlockId(54)), "/blocks/54?force=true");
    }

    #[rstest]
    fn trash_block(endpoint: BlocksRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.trash(&BlockId(54)), "/blocks/54?force=false");
    }

    #[rstest]
    fn update_block(endpoint: BlocksRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.update(&BlockId(54)), "/blocks/54");
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> BlocksRequestEndpoint {
        BlocksRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
