use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{
    SparseField,
    categories::{
        CategoryId, CategoryListParams, SparseCategoryFieldWithEditContext,
        SparseCategoryFieldWithEmbedContext, SparseCategoryFieldWithViewContext,
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum CategoriesRequest {
    #[contextual_paged(url = "/categories", params = &CategoryListParams, output = Vec<crate::categories::SparseCategory>, filter_by = crate::categories::SparseCategoryField)]
    List,
    #[contextual_get(url = "/categories/<category_id>", output = crate::categories::SparseCategory, filter_by = crate::categories::SparseCategoryField)]
    Retrieve,
    #[post(url = "/categories", params = &crate::categories::CategoryCreateParams, output = crate::categories::CategoryWithEditContext)]
    Create,
    #[delete(url = "/categories/<category_id>", output = crate::categories::CategoryDeleteResponse)]
    Delete,
    #[post(url = "/categories/<category_id>", params = &crate::categories::CategoryUpdateParams, output = crate::categories::CategoryWithEditContext)]
    Update,
}

impl DerivedRequest for CategoriesRequest {
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

super::macros::default_sparse_field_implementation_from_field_name!(
    SparseCategoryFieldWithEditContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparseCategoryFieldWithEmbedContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparseCategoryFieldWithViewContext
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        WpApiParamOrder,
        categories::{CategoryId, WpApiParamCategoriesOrderBy},
        generate,
        posts::PostId,
        request::endpoint::{
            ApiBaseUrl,
            tests::{fixture_api_base_url, validate_wp_v2_endpoint},
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(CategoryListParams::default(), "")]
    #[case(generate!(CategoryListParams, (page, Some(2))), "page=2")]
    #[case(generate!(CategoryListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(CategoryListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(CategoryListParams, (exclude, vec![CategoryId(1), CategoryId(2)])), "exclude=1%2C2")]
    #[case(generate!(CategoryListParams, (include, vec![CategoryId(1), CategoryId(2)])), "include=1%2C2")]
    #[case(generate!(CategoryListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(CategoryListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(CategoryListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::Id))), "orderby=id")]
    #[case(generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::Include))), "orderby=include")]
    #[case(generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::Name))), "orderby=name")]
    #[case(generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::TermGroup))), "orderby=term_group")]
    #[case(generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::Description))), "orderby=description")]
    #[case(generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::Count))), "orderby=count")]
    #[case(generate!(CategoryListParams, (hide_empty, Some(true))), "hide_empty=true")]
    #[case(generate!(CategoryListParams, (parent, Some(CategoryId(3)))), "parent=3")]
    #[case(generate!(CategoryListParams, (post, Some(PostId(3)))), "post=3")]
    #[case(generate!(CategoryListParams, (slug, vec!["slug_1".to_string(), "slug_2".to_string()])), "slug=slug_1%2Cslug_2")]
    // TODO
    #[case(
        category_list_params_with_all_fields(),
        EXPECTED_QUERY_PAIRS_FOR_CATEGORY_LIST_PARAMS_WITH_ALL_FIELDS
    )]
    fn list_categories(
        endpoint: CategoriesRequestEndpoint,
        #[case] params: CategoryListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/categories?context={}", context)
            } else {
                format!(
                    "/categories?context={}&{}",
                    context, expected_additional_params
                )
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
    #[case(CategoryListParams::default(), &[], "/categories?context=edit&_fields=")]
    #[case(generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::Id))), &[SparseCategoryFieldWithEditContext::Count], "/categories?context=edit&orderby=id&_fields=count")]
    #[case(category_list_params_with_all_fields(), ALL_SPARSE_CATEGORY_FIELDS_WITH_EDIT_CONTEXT, &format!("/categories?context=edit&{}&{}", EXPECTED_QUERY_PAIRS_FOR_CATEGORY_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_CATEGORY_FIELDS_WITH_EDIT_CONTEXT))]
    fn filter_list_categories_with_edit_context(
        endpoint: CategoriesRequestEndpoint,
        #[case] params: CategoryListParams,
        #[case] fields: &[SparseCategoryFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(CategoryListParams::default(), &[], "/categories?context=embed&_fields=")]
    #[case(generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::Slug))), &[SparseCategoryFieldWithEmbedContext::Link], "/categories?context=embed&orderby=slug&_fields=link")]
    #[case(category_list_params_with_all_fields(), ALL_SPARSE_CATEGORY_FIELDS_WITH_EMBED_CONTEXT, &format!("/categories?context=embed&{}&{}", EXPECTED_QUERY_PAIRS_FOR_CATEGORY_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_CATEGORY_FIELDS_WITH_EMBED_CONTEXT))]
    fn filter_list_categories_with_embed_context(
        endpoint: CategoriesRequestEndpoint,
        #[case] params: CategoryListParams,
        #[case] fields: &[SparseCategoryFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_embed_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(CategoryListParams::default(), &[], "/categories?context=view&_fields=")]
    #[case(generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::Include))), &[SparseCategoryFieldWithViewContext::Description], "/categories?context=view&orderby=include&_fields=description")]
    #[case(category_list_params_with_all_fields(), ALL_SPARSE_CATEGORY_FIELDS_WITH_VIEW_CONTEXT, &format!("/categories?context=view&{}&{}", EXPECTED_QUERY_PAIRS_FOR_CATEGORY_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_CATEGORY_FIELDS_WITH_VIEW_CONTEXT))]
    fn filter_list_categories_with_view_context(
        endpoint: CategoriesRequestEndpoint,
        #[case] params: CategoryListParams,
        #[case] fields: &[SparseCategoryFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&params, fields),
            expected_path,
        );
    }

    const EXPECTED_QUERY_PAIRS_FOR_CATEGORY_LIST_PARAMS_WITH_ALL_FIELDS: &str = "page=11&per_page=22&search=s_q&exclude=1111%2C1112&include=2111%2C2112&offset=11111&order=desc&orderby=slug&hide_empty=true&parent=33333&post=44444&slug=slug_1%2Cslug_2";
    fn category_list_params_with_all_fields() -> CategoryListParams {
        CategoryListParams {
            page: Some(11),
            per_page: Some(22),
            search: Some("s_q".to_string()),
            exclude: vec![CategoryId(1111), CategoryId(1112)],
            include: vec![CategoryId(2111), CategoryId(2112)],
            offset: Some(11111),
            order: Some(WpApiParamOrder::Desc),
            orderby: Some(WpApiParamCategoriesOrderBy::Slug),
            hide_empty: Some(true),
            parent: Some(CategoryId(33333)),
            post: Some(PostId(44444)),
            slug: vec!["slug_1".to_string(), "slug_2".to_string()],
        }
    }

    #[rstest]
    fn retrieve_category(endpoint: CategoriesRequestEndpoint) {
        let category_id = CategoryId(54);
        let expected_path = |context: &str| format!("/categories/54?context={}", context);
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&category_id),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&category_id),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&category_id),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(&[], "/categories/54?context=view&_fields=")]
    #[case(&[SparseCategoryFieldWithViewContext::Count], "/categories/54?context=view&_fields=count")]
    #[case(ALL_SPARSE_CATEGORY_FIELDS_WITH_VIEW_CONTEXT, &format!("/categories/54?context=view&{}", EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_CATEGORY_FIELDS_WITH_VIEW_CONTEXT))]
    fn filter_retrieve_category_with_view_context(
        endpoint: CategoriesRequestEndpoint,
        #[case] fields: &[SparseCategoryFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(&CategoryId(54), fields),
            expected_path,
        );
    }

    #[rstest]
    fn create_category(endpoint: CategoriesRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(), "/categories");
    }

    #[rstest]
    fn delete_category(endpoint: CategoriesRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.delete(&CategoryId(54)),
            "/categories/54?force=true",
        );
    }

    #[rstest]
    fn update_category(endpoint: CategoriesRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.update(&CategoryId(54)), "/categories/54");
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_CATEGORY_FIELDS_WITH_EDIT_CONTEXT: &str =
        "_fields=id%2Ccount%2Cdescription%2Clink%2Cname%2Cslug%2Ctaxonomy%2Cparent";
    const ALL_SPARSE_CATEGORY_FIELDS_WITH_EDIT_CONTEXT: &[SparseCategoryFieldWithEditContext; 8] =
        &[
            SparseCategoryFieldWithEditContext::Id,
            SparseCategoryFieldWithEditContext::Count,
            SparseCategoryFieldWithEditContext::Description,
            SparseCategoryFieldWithEditContext::Link,
            SparseCategoryFieldWithEditContext::Name,
            SparseCategoryFieldWithEditContext::Slug,
            SparseCategoryFieldWithEditContext::Taxonomy,
            SparseCategoryFieldWithEditContext::Parent,
        ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_CATEGORY_FIELDS_WITH_EMBED_CONTEXT: &str =
        "_fields=id%2Clink%2Cname%2Cslug%2Ctaxonomy";
    const ALL_SPARSE_CATEGORY_FIELDS_WITH_EMBED_CONTEXT: &[SparseCategoryFieldWithEmbedContext; 5] =
        &[
            SparseCategoryFieldWithEmbedContext::Id,
            SparseCategoryFieldWithEmbedContext::Link,
            SparseCategoryFieldWithEmbedContext::Name,
            SparseCategoryFieldWithEmbedContext::Slug,
            SparseCategoryFieldWithEmbedContext::Taxonomy,
        ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_CATEGORY_FIELDS_WITH_VIEW_CONTEXT: &str =
        "_fields=id%2Ccount%2Cdescription%2Clink%2Cname%2Cslug%2Ctaxonomy%2Cparent";
    const ALL_SPARSE_CATEGORY_FIELDS_WITH_VIEW_CONTEXT: &[SparseCategoryFieldWithViewContext; 8] =
        &[
            SparseCategoryFieldWithViewContext::Id,
            SparseCategoryFieldWithViewContext::Count,
            SparseCategoryFieldWithViewContext::Description,
            SparseCategoryFieldWithViewContext::Link,
            SparseCategoryFieldWithViewContext::Name,
            SparseCategoryFieldWithViewContext::Slug,
            SparseCategoryFieldWithViewContext::Taxonomy,
            SparseCategoryFieldWithViewContext::Parent,
        ];

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> CategoriesRequestEndpoint {
        CategoriesRequestEndpoint::new(fixture_api_base_url)
    }
}
