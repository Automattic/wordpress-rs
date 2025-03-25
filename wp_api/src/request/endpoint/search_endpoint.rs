use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{
    SparseField,
    search_results::{
        SparseSearchResultFieldWithEmbedContext, SparseSearchResultFieldWithViewContext,
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SearchRequest {
    #[contextual_paged(url = "/search", params = &crate::search_results::SearchListParams, output = Vec<crate::search_results::SparseSearchResult>, filter_by = crate::search_results::SparseSearchResultField, available_contexts = "embed,view")]
    List,
}

impl DerivedRequest for SearchRequest {
    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

impl SparseField for SparseSearchResultFieldWithEmbedContext {
    fn as_str(&self) -> &str {
        match self {
            Self::ObjectType => "type",
            Self::ObjectSubtype => "subtype",
            _ => self.as_field_name(),
        }
    }
}

impl SparseField for SparseSearchResultFieldWithViewContext {
    fn as_str(&self) -> &str {
        match self {
            Self::ObjectType => "type",
            Self::ObjectSubtype => "subtype",
            _ => self.as_field_name(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParsedUrl;
    use crate::{
        generate,
        request::endpoint::tests::{fixture_api_base_url, validate_wp_v2_endpoint},
        search_results::{SearchListParams, SearchResultSubtype, SearchResultType},
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(SearchListParams::default(), "")]
    #[case(generate!(SearchListParams, (page, Some(2))), "page=2")]
    #[case(generate!(SearchListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(SearchListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(SearchListParams, (object_type, Some(SearchResultType::Post))), "type=post")]
    #[case(generate!(SearchListParams, (object_type, Some(SearchResultType::Term))), "type=term")]
    #[case(generate!(SearchListParams, (object_type, Some(SearchResultType::PostFormat))), "type=post-format")]
    #[case(generate!(SearchListParams, (object_subtype, Some(SearchResultSubtype::Post))), "subtype=post")]
    #[case(generate!(SearchListParams, (object_subtype, Some(SearchResultSubtype::Page))), "subtype=page")]
    #[case(generate!(SearchListParams, (object_subtype, Some(SearchResultSubtype::Category))), "subtype=category")]
    #[case(generate!(SearchListParams, (object_subtype, Some(SearchResultSubtype::PostTag))), "subtype=post_tag")]
    #[case(generate!(SearchListParams, (exclude, vec![1, 2])), "exclude=1%2C2")]
    #[case(generate!(SearchListParams, (include, vec![1, 2])), "include=1%2C2")]
    #[case(
        search_list_params_with_all_fields(),
        EXPECTED_QUERY_PAIRS_FOR_SEARCH_LIST_PARAMS_WITH_ALL_FIELDS
    )]
    fn list_search(
        endpoint: SearchRequestEndpoint,
        #[case] params: SearchListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/search?context={}", context)
            } else {
                format!("/search?context={}&{}", context, expected_additional_params)
            }
        };
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
    #[case(SearchListParams::default(), &[], "/search?context=embed&_fields=")]
    #[case(search_list_params_with_all_fields(), ALL_SPARSE_SEARCH_RESULT_FIELDS_WITH_EMBED_CONTEXT, &format!("/search?context=embed&{}&{}", EXPECTED_QUERY_PAIRS_FOR_SEARCH_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_SEARCH_RESULT_FIELDS_WITH_EMBED_CONTEXT))]
    fn filter_list_search_with_embed_context(
        endpoint: SearchRequestEndpoint,
        #[case] params: SearchListParams,
        #[case] fields: &[SparseSearchResultFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_embed_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(SearchListParams::default(), &[], "/search?context=view&_fields=")]
    #[case(search_list_params_with_all_fields(), ALL_SPARSE_SEARCH_RESULT_FIELDS_WITH_VIEW_CONTEXT, &format!("/search?context=view&{}&{}", EXPECTED_QUERY_PAIRS_FOR_SEARCH_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_SEARCH_RESULT_FIELDS_WITH_VIEW_CONTEXT))]
    fn filter_list_search_with_view_context(
        endpoint: SearchRequestEndpoint,
        #[case] params: SearchListParams,
        #[case] fields: &[SparseSearchResultFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&params, fields),
            expected_path,
        );
    }

    const EXPECTED_QUERY_PAIRS_FOR_SEARCH_LIST_PARAMS_WITH_ALL_FIELDS: &str = "page=11&per_page=22&search=s_q&type=term&subtype=category&exclude=1111%2C1112&include=2111%2C2112";
    fn search_list_params_with_all_fields() -> SearchListParams {
        SearchListParams {
            page: Some(11),
            per_page: Some(22),
            search: Some("s_q".to_string()),
            object_type: Some(SearchResultType::Term),
            object_subtype: Some(SearchResultSubtype::Category),
            exclude: vec![1111, 1112],
            include: vec![2111, 2112],
        }
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_SEARCH_RESULT_FIELDS_WITH_EMBED_CONTEXT: &str =
        "_fields=id%2Ctitle%2Curl%2Ctype%2Csubtype";
    const ALL_SPARSE_SEARCH_RESULT_FIELDS_WITH_EMBED_CONTEXT: &[SparseSearchResultFieldWithEmbedContext;
         5] = &[
        SparseSearchResultFieldWithEmbedContext::Id,
        SparseSearchResultFieldWithEmbedContext::Title,
        SparseSearchResultFieldWithEmbedContext::Url,
        SparseSearchResultFieldWithEmbedContext::ObjectType,
        SparseSearchResultFieldWithEmbedContext::ObjectSubtype,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_SEARCH_RESULT_FIELDS_WITH_VIEW_CONTEXT: &str =
        "_fields=id%2Ctitle%2Curl%2Ctype%2Csubtype";
    const ALL_SPARSE_SEARCH_RESULT_FIELDS_WITH_VIEW_CONTEXT: &[SparseSearchResultFieldWithViewContext;
         5] = &[
        SparseSearchResultFieldWithViewContext::Id,
        SparseSearchResultFieldWithViewContext::Title,
        SparseSearchResultFieldWithViewContext::Url,
        SparseSearchResultFieldWithViewContext::ObjectType,
        SparseSearchResultFieldWithViewContext::ObjectSubtype,
    ];

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ParsedUrl>) -> SearchRequestEndpoint {
        SearchRequestEndpoint::new(fixture_api_base_url)
    }
}
