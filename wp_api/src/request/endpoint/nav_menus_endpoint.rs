use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::nav_menus::NavMenuId;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum NavMenusRequest {
    #[contextual_get(url = "/menus", params = &crate::nav_menus::NavMenuListParams, output = Vec<crate::nav_menus::SparseNavMenu>, filter_by = crate::nav_menus::SparseNavMenuField)]
    List,
    #[contextual_get(url = "/menus/<nav_menu_id>", output = crate::nav_menus::SparseNavMenu, filter_by = crate::nav_menus::SparseNavMenuField)]
    Retrieve,
    #[post(url = "/menus", params = &crate::nav_menus::NavMenuCreateParams, output = crate::nav_menus::NavMenuWithEditContext)]
    Create,
    #[delete(url = "/menus/<nav_menu_id>", output = crate::nav_menus::NavMenuDeleteResponse)]
    Delete,
    #[post(url = "/menus/<nav_menu_id>", params = &crate::nav_menus::NavMenuUpdateParams, output = crate::nav_menus::NavMenuWithEditContext)]
    Update,
}

impl DerivedRequest for NavMenusRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            NavMenusRequest::Delete => vec![("force", true.to_string())],
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
        nav_menus::{
            NavMenuListParams, SparseNavMenuFieldWithEditContext,
            SparseNavMenuFieldWithEmbedContext, SparseNavMenuFieldWithViewContext,
            WpApiParamNavMenusOrderBy,
        },
        posts::PostId,
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn create_nav_menu(endpoint: NavMenusRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(), "/menus");
    }

    #[rstest]
    fn delete_nav_menu(endpoint: NavMenusRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.delete(&NavMenuId(54)), "/menus/54?force=true");
    }

    #[rstest]
    #[case(NavMenuListParams::default(), "")]
    #[case(generate!(NavMenuListParams, (page, Some(2))), "page=2")]
    #[case(generate!(NavMenuListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(NavMenuListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(NavMenuListParams, (exclude, vec![NavMenuId(1), NavMenuId(2)])), "exclude=1%2C2")]
    #[case(generate!(NavMenuListParams, (include, vec![NavMenuId(1), NavMenuId(2)])), "include=1%2C2")]
    #[case(generate!(NavMenuListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(NavMenuListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(NavMenuListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Id))), "orderby=id")]
    #[case(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Include))), "orderby=include")]
    #[case(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Name))), "orderby=name")]
    #[case(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::TermGroup))), "orderby=term_group")]
    #[case(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Description))), "orderby=description")]
    #[case(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Count))), "orderby=count")]
    #[case(generate!(NavMenuListParams, (hide_empty, Some(false))), "hide_empty=false")]
    #[case(generate!(NavMenuListParams, (hide_empty, Some(true))), "hide_empty=true")]
    #[case(generate!(NavMenuListParams, (post, Some(PostId(123)))), "post=123")]
    #[case(generate!(NavMenuListParams, (slug, vec!["primary".to_string(), "footer".to_string()])), "slug=primary%2Cfooter")]
    #[case(
        nav_menu_list_params_with_all_fields(),
        &expected_query_pairs_for_nav_menu_list_params_with_all_fields()
    )]
    fn list_nav_menus(
        endpoint: NavMenusRequestEndpoint,
        #[case] params: NavMenuListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/menus?context={context}")
            } else {
                format!("/menus?context={context}&{expected_additional_params}")
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
    #[case(NavMenuListParams::default(), &[], "/menus?context=edit&_fields=")]
    #[case(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Name))), &[SparseNavMenuFieldWithEditContext::Name], "/menus?context=edit&orderby=name&_fields=name")]
    #[case(nav_menu_list_params_with_all_fields(), ALL_SPARSE_NAV_MENU_FIELDS_WITH_EDIT_CONTEXT, &format!("/menus?context=edit&{}&{}", expected_query_pairs_for_nav_menu_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_FIELDS_WITH_EDIT_CONTEXT))]
    fn filter_list_nav_menu_with_edit_context(
        endpoint: NavMenusRequestEndpoint,
        #[case] params: NavMenuListParams,
        #[case] fields: &[SparseNavMenuFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(NavMenuListParams::default(), &[], "/menus?context=embed&_fields=")]
    #[case(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Name))), &[SparseNavMenuFieldWithEmbedContext::Name], "/menus?context=embed&orderby=name&_fields=name")]
    #[case(nav_menu_list_params_with_all_fields(), ALL_SPARSE_NAV_MENU_FIELDS_WITH_EMBED_CONTEXT, &format!("/menus?context=embed&{}&{}", expected_query_pairs_for_nav_menu_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_FIELDS_WITH_EMBED_CONTEXT))]
    fn filter_list_nav_menu_with_embed_context(
        endpoint: NavMenusRequestEndpoint,
        #[case] params: NavMenuListParams,
        #[case] fields: &[SparseNavMenuFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_embed_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(NavMenuListParams::default(), &[], "/menus?context=view&_fields=")]
    #[case(generate!(NavMenuListParams, (orderby, Some(WpApiParamNavMenusOrderBy::Name))), &[SparseNavMenuFieldWithViewContext::Name], "/menus?context=view&orderby=name&_fields=name")]
    #[case(nav_menu_list_params_with_all_fields(), ALL_SPARSE_NAV_MENU_FIELDS_WITH_VIEW_CONTEXT, &format!("/menus?context=view&{}&{}", expected_query_pairs_for_nav_menu_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_FIELDS_WITH_VIEW_CONTEXT))]
    fn filter_list_nav_menu_with_view_context(
        endpoint: NavMenusRequestEndpoint,
        #[case] params: NavMenuListParams,
        #[case] fields: &[SparseNavMenuFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    fn retrieve_nav_menu(endpoint: NavMenusRequestEndpoint) {
        let nav_menu_id = NavMenuId(54);
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&nav_menu_id),
            "/menus/54?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&nav_menu_id),
            "/menus/54?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&nav_menu_id),
            "/menus/54?context=view",
        );
    }

    #[rstest]
    #[case(&[], "/menus/54?context=edit&_fields=")]
    #[case(&[SparseNavMenuFieldWithEditContext::Name], "/menus/54?context=edit&_fields=name")]
    #[case(ALL_SPARSE_NAV_MENU_FIELDS_WITH_EDIT_CONTEXT, &format!("/menus/54?context=edit&{}", EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_FIELDS_WITH_EDIT_CONTEXT))]
    fn filter_retrieve_nav_menu_with_edit_context(
        endpoint: NavMenusRequestEndpoint,
        #[case] fields: &[SparseNavMenuFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_edit_context(&NavMenuId(54), fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(&[], "/menus/54?context=embed&_fields=")]
    #[case(&[SparseNavMenuFieldWithEmbedContext::Name], "/menus/54?context=embed&_fields=name")]
    #[case(ALL_SPARSE_NAV_MENU_FIELDS_WITH_EMBED_CONTEXT, &format!("/menus/54?context=embed&{}", EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_FIELDS_WITH_EMBED_CONTEXT))]
    fn filter_retrieve_nav_menu_with_embed_context(
        endpoint: NavMenusRequestEndpoint,
        #[case] fields: &[SparseNavMenuFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_embed_context(&NavMenuId(54), fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(&[], "/menus/54?context=view&_fields=")]
    #[case(&[SparseNavMenuFieldWithViewContext::Name], "/menus/54?context=view&_fields=name")]
    #[case(ALL_SPARSE_NAV_MENU_FIELDS_WITH_VIEW_CONTEXT, &format!("/menus/54?context=view&{}", EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_FIELDS_WITH_VIEW_CONTEXT))]
    fn filter_retrieve_nav_menu_with_view_context(
        endpoint: NavMenusRequestEndpoint,
        #[case] fields: &[SparseNavMenuFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(&NavMenuId(54), fields),
            expected_path,
        );
    }

    #[rstest]
    fn update_nav_menu(endpoint: NavMenusRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.update(&NavMenuId(54)), "/menus/54");
    }

    const ALL_SPARSE_NAV_MENU_FIELDS_WITH_EDIT_CONTEXT: &[SparseNavMenuFieldWithEditContext] = &[
        SparseNavMenuFieldWithEditContext::Id,
        SparseNavMenuFieldWithEditContext::Description,
        SparseNavMenuFieldWithEditContext::Name,
        SparseNavMenuFieldWithEditContext::Slug,
        SparseNavMenuFieldWithEditContext::Locations,
        SparseNavMenuFieldWithEditContext::AutoAdd,
    ];

    const ALL_SPARSE_NAV_MENU_FIELDS_WITH_EMBED_CONTEXT: &[SparseNavMenuFieldWithEmbedContext] = &[
        SparseNavMenuFieldWithEmbedContext::Id,
        SparseNavMenuFieldWithEmbedContext::Name,
        SparseNavMenuFieldWithEmbedContext::Slug,
    ];

    const ALL_SPARSE_NAV_MENU_FIELDS_WITH_VIEW_CONTEXT: &[SparseNavMenuFieldWithViewContext] = &[
        SparseNavMenuFieldWithViewContext::Id,
        SparseNavMenuFieldWithViewContext::Description,
        SparseNavMenuFieldWithViewContext::Name,
        SparseNavMenuFieldWithViewContext::Slug,
        SparseNavMenuFieldWithViewContext::Locations,
        SparseNavMenuFieldWithViewContext::AutoAdd,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_FIELDS_WITH_EDIT_CONTEXT: &str =
        "_fields=id%2Cdescription%2Cname%2Cslug%2Clocations%2Cauto_add";
    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_FIELDS_WITH_EMBED_CONTEXT: &str =
        "_fields=id%2Cname%2Cslug";
    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_FIELDS_WITH_VIEW_CONTEXT: &str =
        "_fields=id%2Cdescription%2Cname%2Cslug%2Clocations%2Cauto_add";

    fn expected_query_pairs_for_nav_menu_list_params_with_all_fields() -> String {
        "page=2&per_page=2&search=foo&exclude=1%2C2&include=1%2C2&offset=2&order=asc&orderby=name&hide_empty=false&post=123&slug=primary%2Cfooter".to_string()
    }

    fn nav_menu_list_params_with_all_fields() -> NavMenuListParams {
        NavMenuListParams {
            page: Some(2),
            per_page: Some(2),
            search: Some("foo".to_string()),
            exclude: vec![NavMenuId(1), NavMenuId(2)],
            include: vec![NavMenuId(1), NavMenuId(2)],
            offset: Some(2),
            order: Some(WpApiParamOrder::Asc),
            orderby: Some(WpApiParamNavMenusOrderBy::Name),
            hide_empty: Some(false),
            post: Some(PostId(123)),
            slug: vec!["primary".to_string(), "footer".to_string()],
        }
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> NavMenusRequestEndpoint {
        NavMenusRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
