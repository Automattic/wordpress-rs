use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::nav_menu_items::NavMenuItemId;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum NavMenuItemsRequest {
    #[contextual_paged(url = "/menu-items", params = &crate::nav_menu_items::NavMenuItemListParams, output = Vec<crate::nav_menu_items::SparseNavMenuItem>, filter_by = crate::nav_menu_items::SparseNavMenuItemField)]
    List,
    #[contextual_get(url = "/menu-items/<nav_menu_item_id>", output = crate::nav_menu_items::SparseNavMenuItem, filter_by = crate::nav_menu_items::SparseNavMenuItemField)]
    Retrieve,
    #[post(url = "/menu-items", params = &crate::nav_menu_items::NavMenuItemCreateParams, output = crate::nav_menu_items::NavMenuItemWithEditContext)]
    Create,
    #[delete(url = "/menu-items/<nav_menu_item_id>", output = crate::nav_menu_items::NavMenuItemDeleteResponse)]
    Delete,
    #[post(url = "/menu-items/<nav_menu_item_id>", params = &crate::nav_menu_items::NavMenuItemUpdateParams, output = crate::nav_menu_items::NavMenuItemWithEditContext)]
    Update,
}

impl DerivedRequest for NavMenuItemsRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            // Trashing a nav menu item is not supported:
            // https://github.com/WordPress/WordPress/blob/b27e369cb25445784bc014d6fa731558beaf320d/wp-includes/rest-api/endpoints/class-wp-rest-menu-items-controller.php#L298-L302
            NavMenuItemsRequest::Delete => vec![("force", true.to_string())],
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
        nav_menu_items::{
            NavMenuItemListParams, NavMenuItemStatus, SparseNavMenuItemFieldWithEditContext,
            SparseNavMenuItemFieldWithEmbedContext, SparseNavMenuItemFieldWithViewContext,
        },
        nav_menus::NavMenuId,
        posts::{WpApiParamPostsOrderBy, WpApiParamPostsSearchColumn, WpApiParamPostsTaxRelation},
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
    fn create_nav_menu_item(endpoint: NavMenuItemsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(), "/menu-items");
    }

    #[rstest]
    fn delete_nav_menu_item(endpoint: NavMenuItemsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.delete(&NavMenuItemId(54)),
            "/menu-items/54?force=true",
        );
    }

    #[rstest]
    #[case(NavMenuItemListParams::default(), "")]
    #[case(generate!(NavMenuItemListParams, (page, Some(2))), "page=2")]
    #[case(generate!(NavMenuItemListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(NavMenuItemListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(NavMenuItemListParams, (after, unit_test_example_date_string_as_option())), &unit_test_example_date_string_as_query_value("after"))]
    #[case(generate!(NavMenuItemListParams, (modified_after, unit_test_example_date_string_as_option())), &unit_test_example_date_string_as_query_value("modified_after"))]
    #[case(generate!(NavMenuItemListParams, (before, unit_test_example_date_string_as_option())), &unit_test_example_date_string_as_query_value("before"))]
    #[case(generate!(NavMenuItemListParams, (modified_before, unit_test_example_date_string_as_option())), &unit_test_example_date_string_as_query_value("modified_before"))]
    #[case(generate!(NavMenuItemListParams, (exclude, vec![NavMenuItemId(1), NavMenuItemId(2)])), "exclude=1%2C2")]
    #[case(generate!(NavMenuItemListParams, (include, vec![NavMenuItemId(1), NavMenuItemId(2)])), "include=1%2C2")]
    #[case(generate!(NavMenuItemListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(NavMenuItemListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(NavMenuItemListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), "orderby=author")]
    #[case(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::Date))), "orderby=date")]
    #[case(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::Id))), "orderby=id")]
    #[case(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::Include))), "orderby=include")]
    #[case(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::Modified))), "orderby=modified")]
    #[case(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::Parent))), "orderby=parent")]
    #[case(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::Relevance))), "orderby=relevance")]
    #[case(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::Title))), "orderby=title")]
    #[case(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::MenuOrder))), "orderby=menu_order")]
    #[case(generate!(NavMenuItemListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent])), "search_columns=post_content")]
    #[case(generate!(NavMenuItemListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostExcerpt])), "search_columns=post_excerpt")]
    #[case(generate!(NavMenuItemListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostTitle])), "search_columns=post_title")]
    #[case(generate!(NavMenuItemListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent, WpApiParamPostsSearchColumn::PostExcerpt, WpApiParamPostsSearchColumn::PostTitle])), "search_columns=post_content%2Cpost_excerpt%2Cpost_title")]
    #[case(generate!(NavMenuItemListParams, (slug, vec!["foo".to_string(), "bar".to_string()])), "slug=foo%2Cbar")]
    #[case(generate!(NavMenuItemListParams, (status, vec![NavMenuItemStatus::Draft])), "status=draft")]
    #[case(generate!(NavMenuItemListParams, (status, vec![NavMenuItemStatus::Publish])), "status=publish")]
    #[case(generate!(NavMenuItemListParams, (status, vec![NavMenuItemStatus::Draft, NavMenuItemStatus::Publish])), "status=draft%2Cpublish")]
    #[case(generate!(NavMenuItemListParams, (tax_relation, Some(WpApiParamPostsTaxRelation::And))), "tax_relation=AND")]
    #[case(generate!(NavMenuItemListParams, (tax_relation, Some(WpApiParamPostsTaxRelation::Or))), "tax_relation=OR")]
    #[case(generate!(NavMenuItemListParams, (menus, vec![NavMenuId(179)])), "menus=179")]
    #[case(generate!(NavMenuItemListParams, (menus, vec![NavMenuId(179), NavMenuId(180)])), "menus=179%2C180")]
    #[case(generate!(NavMenuItemListParams, (menus_exclude, vec![NavMenuId(179)])), "menus_exclude=179")]
    #[case(generate!(NavMenuItemListParams, (menus_exclude, vec![NavMenuId(179), NavMenuId(180)])), "menus_exclude=179%2C180")]
    #[case(generate!(NavMenuItemListParams, (menu_order, Some(1))), "menu_order=1")]
    #[case(
        nav_menu_item_list_params_with_all_fields(),
        &expected_query_pairs_for_nav_menu_item_list_params_with_all_fields()
    )]
    fn list_nav_menu_items(
        endpoint: NavMenuItemsRequestEndpoint,
        #[case] params: NavMenuItemListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/menu-items?context={context}")
            } else {
                format!("/menu-items?context={context}&{expected_additional_params}")
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
    #[case(NavMenuItemListParams::default(), &[], "/menu-items?context=edit&_fields=")]
    #[case(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), &[SparseNavMenuItemFieldWithEditContext::Title], "/menu-items?context=edit&orderby=author&_fields=title")]
    #[case(nav_menu_item_list_params_with_all_fields(), ALL_SPARSE_NAV_MENU_ITEM_FIELDS_WITH_EDIT_CONTEXT, &format!("/menu-items?context=edit&{}&{}", expected_query_pairs_for_nav_menu_item_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_ITEM_FIELDS_WITH_EDIT_CONTEXT))]
    fn filter_list_nav_menu_item_with_edit_context(
        endpoint: NavMenuItemsRequestEndpoint,
        #[case] params: NavMenuItemListParams,
        #[case] fields: &[SparseNavMenuItemFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(NavMenuItemListParams::default(), &[], "/menu-items?context=embed&_fields=")]
    #[case(generate!(NavMenuItemListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), &[SparseNavMenuItemFieldWithEmbedContext::Title], "/menu-items?context=embed&orderby=author&_fields=title")]
    #[case(nav_menu_item_list_params_with_all_fields(), ALL_SPARSE_NAV_MENU_ITEM_FIELDS_WITH_EMBED_CONTEXT, &format!("/menu-items?context=embed&{}&{}", expected_query_pairs_for_nav_menu_item_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_ITEM_FIELDS_WITH_EMBED_CONTEXT))]
    fn filter_list_nav_menu_item_with_embed_context(
        endpoint: NavMenuItemsRequestEndpoint,
        #[case] params: NavMenuItemListParams,
        #[case] fields: &[SparseNavMenuItemFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_embed_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    fn retrieve_nav_menu_item(endpoint: NavMenuItemsRequestEndpoint) {
        let nav_menu_item_id = NavMenuItemId(54);
        let expected_path = |context: &str| format!("/menu-items/54?context={context}");
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&nav_menu_item_id),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&nav_menu_item_id),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&nav_menu_item_id),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(&[], "/menu-items/54?context=view&_fields=")]
    #[case(&[SparseNavMenuItemFieldWithViewContext::Title], "/menu-items/54?context=view&_fields=title")]
    #[case(ALL_SPARSE_NAV_MENU_ITEM_FIELDS_WITH_VIEW_CONTEXT, &format!("/menu-items/54?context=view&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_ITEM_FIELDS_WITH_VIEW_CONTEXT}"))]
    fn filter_retrieve_nav_menu_item_with_view_context(
        endpoint: NavMenuItemsRequestEndpoint,
        #[case] fields: &[SparseNavMenuItemFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(&NavMenuItemId(54), fields),
            expected_path,
        );
    }

    #[rstest]
    fn update_nav_menu_item(endpoint: NavMenuItemsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.update(&NavMenuItemId(54)), "/menu-items/54");
    }

    fn expected_query_pairs_for_nav_menu_item_list_params_with_all_fields() -> String {
        let after = unit_test_example_date_string_as_query_value("after");
        let modified_after = unit_test_example_date_string_as_query_value("modified_after");
        let before = unit_test_example_date_string_as_query_value("before");
        let modified_before = unit_test_example_date_string_as_query_value("modified_before");
        format!(
            "page=2&per_page=2&search=foo&{after}&{modified_after}&{before}&{modified_before}&exclude=1%2C2&include=1%2C2&offset=2&order=asc&orderby=author&search_columns=post_content%2Cpost_excerpt%2Cpost_title&slug=foo%2Cbar&status=draft%2Cpublish&tax_relation=AND&menus=179%2C180&menus_exclude=179%2C180&menu_order=1"
        )
    }

    fn nav_menu_item_list_params_with_all_fields() -> NavMenuItemListParams {
        NavMenuItemListParams {
            after: unit_test_example_date_string_as_option(),
            before: unit_test_example_date_string_as_option(),
            exclude: vec![NavMenuItemId(1), NavMenuItemId(2)],
            include: vec![NavMenuItemId(1), NavMenuItemId(2)],
            modified_after: unit_test_example_date_string_as_option(),
            modified_before: unit_test_example_date_string_as_option(),
            offset: Some(2),
            order: Some(WpApiParamOrder::Asc),
            orderby: Some(WpApiParamPostsOrderBy::Author),
            page: Some(2),
            per_page: Some(2),
            search: Some("foo".to_string()),
            search_columns: vec![
                WpApiParamPostsSearchColumn::PostContent,
                WpApiParamPostsSearchColumn::PostExcerpt,
                WpApiParamPostsSearchColumn::PostTitle,
            ],
            slug: vec!["foo".to_string(), "bar".to_string()],
            status: vec![NavMenuItemStatus::Draft, NavMenuItemStatus::Publish],
            tax_relation: Some(WpApiParamPostsTaxRelation::And),
            menus: vec![NavMenuId(179), NavMenuId(180)],
            menus_exclude: vec![NavMenuId(179), NavMenuId(180)],
            menu_order: Some(1),
        }
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_ITEM_FIELDS_WITH_EDIT_CONTEXT: &str = "_fields=title%2Cid%2Ctype_label%2Ctype%2Cstatus%2Cparent%2Cattr_title%2Cclasses%2Cdescription%2Cmenu_order%2Cobject%2Cobject_id%2Ctarget%2Curl%2Cxfn%2Cinvalid%2Cmenus";
    const ALL_SPARSE_NAV_MENU_ITEM_FIELDS_WITH_EDIT_CONTEXT: &[SparseNavMenuItemFieldWithEditContext;
         17] = &[
        SparseNavMenuItemFieldWithEditContext::Title,
        SparseNavMenuItemFieldWithEditContext::Id,
        SparseNavMenuItemFieldWithEditContext::TypeLabel,
        SparseNavMenuItemFieldWithEditContext::ItemType,
        SparseNavMenuItemFieldWithEditContext::Status,
        SparseNavMenuItemFieldWithEditContext::Parent,
        SparseNavMenuItemFieldWithEditContext::AttrTitle,
        SparseNavMenuItemFieldWithEditContext::Classes,
        SparseNavMenuItemFieldWithEditContext::Description,
        SparseNavMenuItemFieldWithEditContext::MenuOrder,
        SparseNavMenuItemFieldWithEditContext::Object,
        SparseNavMenuItemFieldWithEditContext::ObjectId,
        SparseNavMenuItemFieldWithEditContext::Target,
        SparseNavMenuItemFieldWithEditContext::Url,
        SparseNavMenuItemFieldWithEditContext::Xfn,
        SparseNavMenuItemFieldWithEditContext::Invalid,
        SparseNavMenuItemFieldWithEditContext::Menus,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_ITEM_FIELDS_WITH_EMBED_CONTEXT: &str = "_fields=title%2Cid%2Ctype_label%2Ctype%2Cstatus%2Cparent%2Cattr_title%2Cclasses%2Cdescription%2Cmenu_order%2Cobject%2Cobject_id%2Ctarget%2Curl%2Cxfn%2Cinvalid";
    const ALL_SPARSE_NAV_MENU_ITEM_FIELDS_WITH_EMBED_CONTEXT: &[SparseNavMenuItemFieldWithEmbedContext;
         16] = &[
        SparseNavMenuItemFieldWithEmbedContext::Title,
        SparseNavMenuItemFieldWithEmbedContext::Id,
        SparseNavMenuItemFieldWithEmbedContext::TypeLabel,
        SparseNavMenuItemFieldWithEmbedContext::ItemType,
        SparseNavMenuItemFieldWithEmbedContext::Status,
        SparseNavMenuItemFieldWithEmbedContext::Parent,
        SparseNavMenuItemFieldWithEmbedContext::AttrTitle,
        SparseNavMenuItemFieldWithEmbedContext::Classes,
        SparseNavMenuItemFieldWithEmbedContext::Description,
        SparseNavMenuItemFieldWithEmbedContext::MenuOrder,
        SparseNavMenuItemFieldWithEmbedContext::Object,
        SparseNavMenuItemFieldWithEmbedContext::ObjectId,
        SparseNavMenuItemFieldWithEmbedContext::Target,
        SparseNavMenuItemFieldWithEmbedContext::Url,
        SparseNavMenuItemFieldWithEmbedContext::Xfn,
        SparseNavMenuItemFieldWithEmbedContext::Invalid,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_NAV_MENU_ITEM_FIELDS_WITH_VIEW_CONTEXT: &str = "_fields=title%2Cid%2Ctype_label%2Ctype%2Cstatus%2Cparent%2Cattr_title%2Cclasses%2Cdescription%2Cmenu_order%2Cobject%2Cobject_id%2Ctarget%2Curl%2Cxfn%2Cinvalid%2Cmenus";
    const ALL_SPARSE_NAV_MENU_ITEM_FIELDS_WITH_VIEW_CONTEXT: &[SparseNavMenuItemFieldWithViewContext;
         17] = &[
        SparseNavMenuItemFieldWithViewContext::Title,
        SparseNavMenuItemFieldWithViewContext::Id,
        SparseNavMenuItemFieldWithViewContext::TypeLabel,
        SparseNavMenuItemFieldWithViewContext::ItemType,
        SparseNavMenuItemFieldWithViewContext::Status,
        SparseNavMenuItemFieldWithViewContext::Parent,
        SparseNavMenuItemFieldWithViewContext::AttrTitle,
        SparseNavMenuItemFieldWithViewContext::Classes,
        SparseNavMenuItemFieldWithViewContext::Description,
        SparseNavMenuItemFieldWithViewContext::MenuOrder,
        SparseNavMenuItemFieldWithViewContext::Object,
        SparseNavMenuItemFieldWithViewContext::ObjectId,
        SparseNavMenuItemFieldWithViewContext::Target,
        SparseNavMenuItemFieldWithViewContext::Url,
        SparseNavMenuItemFieldWithViewContext::Xfn,
        SparseNavMenuItemFieldWithViewContext::Invalid,
        SparseNavMenuItemFieldWithViewContext::Menus,
    ];

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> NavMenuItemsRequestEndpoint {
        NavMenuItemsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
