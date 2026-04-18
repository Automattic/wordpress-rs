use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::terms::{AnyTermWithEditContext, TermId, TermListParams, TermUpdateParams};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum TermsRequest {
    #[contextual_paged(url = "/<term_endpoint_type>", params = &TermListParams, output = Vec<crate::terms::SparseAnyTerm>, filter_by = crate::terms::SparseAnyTermField)]
    List,
    #[contextual_get(url = "/<term_endpoint_type>/<term_id>", output = crate::terms::SparseAnyTerm, filter_by = crate::terms::SparseAnyTermField)]
    Retrieve,
    #[post(url = "/<term_endpoint_type>", params = &crate::terms::TermCreateParams, output = crate::terms::AnyTermWithEditContext)]
    Create,
    #[delete(url = "/<term_endpoint_type>/<term_id>", output = crate::terms::TermDeleteResponse)]
    Delete,
    #[post(url = "/<term_endpoint_type>/<term_id>", params = &TermUpdateParams, output = AnyTermWithEditContext)]
    Update,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum TermEndpointType {
    Categories,
    Tags,
    #[strum(default)]
    Custom(String),
}

impl DerivedRequest for TermsRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            // The server always returns an error when `force=false`, so a separate `Trash` action
            // is not implemented.
            Self::Delete => vec![("force", true.to_string())],
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
        WpApiParamOrder, generate,
        posts::PostId,
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
        terms::{
            SparseAnyTermFieldWithEditContext, TermId, TermListParams, WpApiParamTermsOrderBy,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(TermListParams::default(), "")]
    #[case(generate!(TermListParams, (page, Some(2))), "page=2")]
    #[case(generate!(TermListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(TermListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(TermListParams, (exclude, vec![TermId(1), TermId(2)])), "exclude=1%2C2")]
    #[case(generate!(TermListParams, (include, vec![TermId(1), TermId(2)])), "include=1%2C2")]
    #[case(generate!(TermListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(TermListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(TermListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Id))), "orderby=id")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Include))), "orderby=include")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Name))), "orderby=name")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::TermGroup))), "orderby=term_group")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Description))), "orderby=description")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Count))), "orderby=count")]
    #[case(generate!(TermListParams, (hide_empty, Some(true))), "hide_empty=true")]
    #[case(generate!(TermListParams, (parent, Some(TermId(3)))), "parent=3")]
    #[case(generate!(TermListParams, (post, Some(PostId(3)))), "post=3")]
    #[case(generate!(TermListParams, (slug, vec!["slug_1".to_string(), "slug_2".to_string()])), "slug=slug_1%2Cslug_2")]
    #[case(
        category_list_params_with_all_fields(),
        EXPECTED_QUERY_PAIRS_FOR_CATEGORY_LIST_PARAMS_WITH_ALL_FIELDS
    )]
    fn list_categories_with_edit_context(
        endpoint: TermsRequestEndpoint,
        #[case] params: TermListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = if expected_additional_params.is_empty() {
            "/categories?context=edit".to_string()
        } else {
            format!("/categories?context=edit&{expected_additional_params}")
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&TermEndpointType::Categories, &params),
            &expected_path,
        );
    }

    #[rstest]
    #[case(TermListParams::default(), "")]
    #[case(generate!(TermListParams, (page, Some(2))), "page=2")]
    #[case(generate!(TermListParams, (per_page, Some(2))), "per_page=2")]
    fn list_categories_with_embed_context(
        endpoint: TermsRequestEndpoint,
        #[case] params: TermListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = if expected_additional_params.is_empty() {
            "/categories?context=embed".to_string()
        } else {
            format!("/categories?context=embed&{expected_additional_params}")
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&TermEndpointType::Categories, &params),
            &expected_path,
        );
    }

    #[rstest]
    #[case(TermListParams::default(), "")]
    #[case(generate!(TermListParams, (page, Some(2))), "page=2")]
    #[case(generate!(TermListParams, (per_page, Some(2))), "per_page=2")]
    fn list_categories_with_view_context(
        endpoint: TermsRequestEndpoint,
        #[case] params: TermListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = if expected_additional_params.is_empty() {
            "/categories?context=view".to_string()
        } else {
            format!("/categories?context=view&{expected_additional_params}")
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&TermEndpointType::Categories, &params),
            &expected_path,
        );
    }

    #[rstest]
    #[case(TermListParams::default(), "")]
    #[case(generate!(TermListParams, (page, Some(2))), "page=2")]
    #[case(generate!(TermListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(TermListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(TermListParams, (exclude, vec![TermId(1), TermId(2)])), "exclude=1%2C2")]
    #[case(generate!(TermListParams, (include, vec![TermId(1), TermId(2)])), "include=1%2C2")]
    #[case(generate!(TermListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(TermListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(TermListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Id))), "orderby=id")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Include))), "orderby=include")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Name))), "orderby=name")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::TermGroup))), "orderby=term_group")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Description))), "orderby=description")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Count))), "orderby=count")]
    #[case(generate!(TermListParams, (hide_empty, Some(true))), "hide_empty=true")]
    #[case(generate!(TermListParams, (post, Some(PostId(3)))), "post=3")]
    #[case(generate!(TermListParams, (slug, vec!["slug_1".to_string(), "slug_2".to_string()])), "slug=slug_1%2Cslug_2")]
    #[case(
        tag_list_params_with_all_fields(),
        EXPECTED_QUERY_PAIRS_FOR_TAG_LIST_PARAMS_WITH_ALL_FIELDS
    )]
    fn list_tags_with_edit_context(
        endpoint: TermsRequestEndpoint,
        #[case] params: TermListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = if expected_additional_params.is_empty() {
            "/tags?context=edit".to_string()
        } else {
            format!("/tags?context=edit&{expected_additional_params}")
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&TermEndpointType::Tags, &params),
            &expected_path,
        );
    }

    #[rstest]
    #[case(TermListParams::default(), &[], "/categories?context=edit&_fields=")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Id))), &[SparseAnyTermFieldWithEditContext::Count], "/categories?context=edit&orderby=id&_fields=count")]
    #[case(category_list_params_with_all_fields(), ALL_SPARSE_TERM_FIELDS_WITH_EDIT_CONTEXT, &format!("/categories?context=edit&{EXPECTED_QUERY_PAIRS_FOR_CATEGORY_LIST_PARAMS_WITH_ALL_FIELDS}&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TERM_FIELDS_WITH_EDIT_CONTEXT}"))]
    fn filter_list_categories_with_edit_context(
        endpoint: TermsRequestEndpoint,
        #[case] params: TermListParams,
        #[case] fields: &[SparseAnyTermFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&TermEndpointType::Categories, &params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(TermListParams::default(), &[], "/tags?context=edit&_fields=")]
    #[case(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Id))), &[SparseAnyTermFieldWithEditContext::Count], "/tags?context=edit&orderby=id&_fields=count")]
    #[case(tag_list_params_with_all_fields(), ALL_SPARSE_TERM_FIELDS_WITH_EDIT_CONTEXT, &format!("/tags?context=edit&{EXPECTED_QUERY_PAIRS_FOR_TAG_LIST_PARAMS_WITH_ALL_FIELDS}&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TERM_FIELDS_WITH_EDIT_CONTEXT}"))]
    fn filter_list_tags_with_edit_context(
        endpoint: TermsRequestEndpoint,
        #[case] params: TermListParams,
        #[case] fields: &[SparseAnyTermFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&TermEndpointType::Tags, &params, fields),
            expected_path,
        );
    }

    #[rstest]
    fn retrieve_category_by_id(endpoint: TermsRequestEndpoint) {
        let term_id = TermId(54);
        let expected_path = |context: &str| format!("/categories/54?context={context}");
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&TermEndpointType::Categories, &term_id),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&TermEndpointType::Categories, &term_id),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&TermEndpointType::Categories, &term_id),
            &expected_path("view"),
        );
    }

    #[rstest]
    fn retrieve_tag_by_id(endpoint: TermsRequestEndpoint) {
        let term_id = TermId(54);
        let expected_path = |context: &str| format!("/tags/54?context={context}");
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&TermEndpointType::Tags, &term_id),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&TermEndpointType::Tags, &term_id),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&TermEndpointType::Tags, &term_id),
            &expected_path("view"),
        );
    }

    #[rstest]
    fn create_category(endpoint: TermsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.create(&TermEndpointType::Categories),
            "/categories",
        );
    }

    #[rstest]
    fn create_tag(endpoint: TermsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(&TermEndpointType::Tags), "/tags");
    }

    #[rstest]
    fn delete_category(endpoint: TermsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.delete(&TermEndpointType::Categories, &TermId(54)),
            "/categories/54?force=true",
        );
    }

    #[rstest]
    fn delete_tag(endpoint: TermsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.delete(&TermEndpointType::Tags, &TermId(54)),
            "/tags/54?force=true",
        );
    }

    #[rstest]
    fn update_category(endpoint: TermsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.update(&TermEndpointType::Categories, &TermId(54)),
            "/categories/54",
        );
    }

    #[rstest]
    fn update_tag(endpoint: TermsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.update(&TermEndpointType::Tags, &TermId(54)),
            "/tags/54",
        );
    }

    const EXPECTED_QUERY_PAIRS_FOR_CATEGORY_LIST_PARAMS_WITH_ALL_FIELDS: &str = "page=11&per_page=22&search=s_q&exclude=1111%2C1112&include=2111%2C2112&offset=11111&order=desc&orderby=slug&hide_empty=true&parent=33333&post=44444&slug=slug_1%2Cslug_2";
    fn category_list_params_with_all_fields() -> TermListParams {
        TermListParams {
            page: Some(11),
            per_page: Some(22),
            search: Some("s_q".to_string()),
            exclude: vec![TermId(1111), TermId(1112)],
            include: vec![TermId(2111), TermId(2112)],
            offset: Some(11111),
            order: Some(WpApiParamOrder::Desc),
            orderby: Some(WpApiParamTermsOrderBy::Slug),
            hide_empty: Some(true),
            parent: Some(TermId(33333)),
            post: Some(PostId(44444)),
            slug: vec!["slug_1".to_string(), "slug_2".to_string()],
        }
    }

    const EXPECTED_QUERY_PAIRS_FOR_TAG_LIST_PARAMS_WITH_ALL_FIELDS: &str = "page=11&per_page=22&search=s_q&exclude=1111%2C1112&include=2111%2C2112&offset=11111&order=desc&orderby=slug&hide_empty=true&post=33333&slug=slug_1%2Cslug_2";
    fn tag_list_params_with_all_fields() -> TermListParams {
        TermListParams {
            page: Some(11),
            per_page: Some(22),
            search: Some("s_q".to_string()),
            exclude: vec![TermId(1111), TermId(1112)],
            include: vec![TermId(2111), TermId(2112)],
            offset: Some(11111),
            order: Some(WpApiParamOrder::Desc),
            orderby: Some(WpApiParamTermsOrderBy::Slug),
            hide_empty: Some(true),
            parent: None,
            post: Some(PostId(33333)),
            slug: vec!["slug_1".to_string(), "slug_2".to_string()],
        }
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TERM_FIELDS_WITH_EDIT_CONTEXT: &str =
        "_fields=id%2Ccount%2Cdescription%2Clink%2Cname%2Cslug%2Ctaxonomy%2Cparent";
    const ALL_SPARSE_TERM_FIELDS_WITH_EDIT_CONTEXT: &[SparseAnyTermFieldWithEditContext; 8] = &[
        SparseAnyTermFieldWithEditContext::Id,
        SparseAnyTermFieldWithEditContext::Count,
        SparseAnyTermFieldWithEditContext::Description,
        SparseAnyTermFieldWithEditContext::Link,
        SparseAnyTermFieldWithEditContext::Name,
        SparseAnyTermFieldWithEditContext::Slug,
        SparseAnyTermFieldWithEditContext::Taxonomy,
        SparseAnyTermFieldWithEditContext::Parent,
    ];

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> TermsRequestEndpoint {
        TermsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
