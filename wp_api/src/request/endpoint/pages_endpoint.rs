use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::pages::{PageId, PageListParams, PageUpdateParams, PageWithEditContext};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum PagesRequest {
    #[contextual_paged(url = "/pages", params = &PageListParams, output = Vec<crate::pages::SparsePage>, filter_by = crate::pages::SparsePageField)]
    List,
    #[contextual_get(url = "/pages/<page_id>", params = &crate::pages::PageRetrieveParams, output = crate::pages::SparsePage, filter_by = crate::pages::SparsePageField)]
    Retrieve,
    #[post(url = "/pages", params = &crate::pages::PageCreateParams, output = crate::pages::PageWithEditContext)]
    Create,
    #[delete(url = "/pages/<page_id>", output = crate::pages::PageDeleteResponse)]
    Delete,
    #[delete(url = "/pages/<page_id>", output = crate::pages::PageWithEditContext)]
    Trash,
    #[post(url = "/pages/<page_id>", params = &PageUpdateParams, output = PageWithEditContext)]
    Update,
}

impl DerivedRequest for PagesRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            PagesRequest::Delete => vec![("force", true.to_string())],
            PagesRequest::Trash => vec![("force", false.to_string())],
            _ => vec![],
        }
    }

    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        UserId, WpApiParamOrder, generate,
        pages::{
            PageRetrieveParams, PageStatus, SparsePageFieldWithEditContext,
            SparsePageFieldWithEmbedContext, SparsePageFieldWithViewContext,
            WpApiParamPagesOrderBy,
        },
        posts::WpApiParamPostsSearchColumn,
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
        unit_test_common::{
            unit_test_example_date_as_option, unit_test_example_date_as_query_value,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn create_page(endpoint: PagesRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(), "/pages");
    }

    #[rstest]
    fn delete_page(endpoint: PagesRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.delete(&PageId(54)), "/pages/54?force=true");
    }

    #[rstest]
    #[case(PageListParams::default(), "")]
    #[case(generate!(PageListParams, (page, Some(2))), "page=2")]
    #[case(generate!(PageListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(PageListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(PageListParams, (after, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("after"))]
    #[case(generate!(PageListParams, (modified_after, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("modified_after"))]
    #[case(generate!(PageListParams, (author, vec![UserId(1), UserId(2)])), "author=1%2C2")]
    #[case(generate!(PageListParams, (author_exclude, vec![UserId(1), UserId(2)])), "author_exclude=1%2C2")]
    #[case(generate!(PageListParams, (before, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("before"))]
    #[case(generate!(PageListParams, (modified_before, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("modified_before"))]
    #[case(generate!(PageListParams, (exclude, vec![PageId(1), PageId(2)])), "exclude=1%2C2")]
    #[case(generate!(PageListParams, (include, vec![PageId(1), PageId(2)])), "include=1%2C2")]
    #[case(generate!(PageListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(PageListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(PageListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Author))), "orderby=author")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Date))), "orderby=date")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Id))), "orderby=id")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Include))), "orderby=include")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::MenuOrder))), "orderby=menu_order")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Modified))), "orderby=modified")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Parent))), "orderby=parent")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Relevance))), "orderby=relevance")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Title))), "orderby=title")]
    #[case(generate!(PageListParams, (slug, vec!["foo".to_string(), "bar".to_string()])), "slug=foo%2Cbar")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Draft])), "status=draft")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Future])), "status=future")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Pending])), "status=pending")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Private])), "status=private")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Publish])), "status=publish")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Custom("foo".to_string())])), "status=foo")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Draft, PageStatus::Future, PageStatus::Pending, PageStatus::Private, PageStatus::Publish, PageStatus::Custom("foo".to_string())])), "status=draft%2Cfuture%2Cpending%2Cprivate%2Cpublish%2Cfoo")]
    #[case(generate!(PageListParams, (parent, Some(PageId(1)))), "parent=1")]
    #[case(generate!(PageListParams, (parent_exclude, vec![PageId(1), PageId(2)])), "parent_exclude=1%2C2")]
    #[case(generate!(PageListParams, (menu_order, Some(1))), "menu_order=1")]
    #[case(
        page_list_params_with_all_fields(),
        &expected_query_pairs_for_page_list_params_with_all_fields()
    )]
    fn list_pages(
        endpoint: PagesRequestEndpoint,
        #[case] params: PageListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/pages?context={context}")
            } else {
                format!("/pages?context={context}&{expected_additional_params}")
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
    #[case(PageListParams::default(), &[], "/pages?context=edit&_fields=")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Author))), &[SparsePageFieldWithEditContext::Author], "/pages?context=edit&orderby=author&_fields=author")]
    #[case(page_list_params_with_all_fields(), ALL_SPARSE_PAGE_FIELDS_WITH_EDIT_CONTEXT, &format!("/pages?context=edit&{}&{}", expected_query_pairs_for_page_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_PAGE_FIELDS_WITH_EDIT_CONTEXT))]
    fn filter_list_page_with_edit_context(
        endpoint: PagesRequestEndpoint,
        #[case] params: PageListParams,
        #[case] fields: &[SparsePageFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(PageListParams::default(), &[], "/pages?context=embed&_fields=")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Author))), &[SparsePageFieldWithEmbedContext::Author], "/pages?context=embed&orderby=author&_fields=author")]
    #[case(page_list_params_with_all_fields(), ALL_SPARSE_PAGE_FIELDS_WITH_EMBED_CONTEXT, &format!("/pages?context=embed&{}&{}", expected_query_pairs_for_page_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_PAGE_FIELDS_WITH_EMBED_CONTEXT))]
    fn filter_list_page_with_embed_context(
        endpoint: PagesRequestEndpoint,
        #[case] params: PageListParams,
        #[case] fields: &[SparsePageFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_embed_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(None, "")]
    #[case(Some("foo"), "password=foo")]
    fn retrieve_page(
        endpoint: PagesRequestEndpoint,
        #[case] password: Option<&str>,
        #[case] expected_additional_params: &str,
    ) {
        let page_id = PageId(54);
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/pages/54?context={context}")
            } else {
                format!("/pages/54?context={context}&{expected_additional_params}")
            }
        };
        let params = PageRetrieveParams {
            password: password.map(|p| p.to_string()),
        };
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&page_id, &params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&page_id, &params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&page_id, &params),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(None, &[], "/pages/54?context=view&_fields=")]
    #[case(Some("foo"), &[SparsePageFieldWithViewContext::Author], "/pages/54?context=view&password=foo&_fields=author")]
    #[case(Some("foo"), ALL_SPARSE_PAGE_FIELDS_WITH_VIEW_CONTEXT, &format!("/pages/54?context=view&password=foo&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_PAGE_FIELDS_WITH_VIEW_CONTEXT}"))]
    fn filter_retrieve_page_with_view_context(
        endpoint: PagesRequestEndpoint,
        #[case] password: Option<&str>,
        #[case] fields: &[SparsePageFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(
                &PageId(54),
                &PageRetrieveParams {
                    password: password.map(|p| p.to_string()),
                },
                fields,
            ),
            expected_path,
        );
    }

    #[rstest]
    fn trash_page(endpoint: PagesRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.trash(&PageId(54)), "/pages/54?force=false");
    }

    #[rstest]
    fn update_page(endpoint: PagesRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.update(&PageId(54)), "/pages/54");
    }

    fn expected_query_pairs_for_page_list_params_with_all_fields() -> String {
        let after = unit_test_example_date_as_query_value("after");
        let modified_after = unit_test_example_date_as_query_value("modified_after");
        let before = unit_test_example_date_as_query_value("before");
        let modified_before = unit_test_example_date_as_query_value("modified_before");
        format!(
            "page=2&per_page=2&search=foo&{after}&{modified_after}&author=1%2C2&author_exclude=1%2C2&{before}&{modified_before}&exclude=1%2C2&include=1%2C2&offset=2&order=asc&orderby=author&search_columns=post_content%2Cpost_excerpt%2Cpost_title&slug=foo%2Cbar&status=draft%2Cfuture%2Cpending%2Cprivate%2Cpublish%2Cfoo&parent=1&parent_exclude=1%2C2&menu_order=1"
        )
    }

    fn page_list_params_with_all_fields() -> PageListParams {
        PageListParams {
            after: unit_test_example_date_as_option(),
            author: vec![UserId(1), UserId(2)],
            author_exclude: vec![UserId(1), UserId(2)],
            before: unit_test_example_date_as_option(),
            exclude: vec![PageId(1), PageId(2)],
            include: vec![PageId(1), PageId(2)],
            modified_after: unit_test_example_date_as_option(),
            modified_before: unit_test_example_date_as_option(),
            offset: Some(2),
            order: Some(WpApiParamOrder::Asc),
            orderby: Some(WpApiParamPagesOrderBy::Author),
            page: Some(2),
            per_page: Some(2),
            search: Some("foo".to_string()),
            search_columns: vec![
                WpApiParamPostsSearchColumn::PostContent,
                WpApiParamPostsSearchColumn::PostExcerpt,
                WpApiParamPostsSearchColumn::PostTitle,
            ],
            slug: vec!["foo".to_string(), "bar".to_string()],
            status: vec![
                PageStatus::Draft,
                PageStatus::Future,
                PageStatus::Pending,
                PageStatus::Private,
                PageStatus::Publish,
                PageStatus::Custom("foo".to_string()),
            ],
            parent: Some(PageId(1)),
            parent_exclude: vec![PageId(1), PageId(2)],
            menu_order: Some(1),
        }
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_PAGE_FIELDS_WITH_EDIT_CONTEXT: &str = "_fields=id%2Cdate%2Cdate_gmt%2Cguid%2Clink%2Cmodified%2Cmodified_gmt%2Cslug%2Cstatus%2Ctype%2Cparent%2Cmenu_order%2Ctitle%2Ccontent%2Cauthor%2Cexcerpt%2Cfeatured_media%2Ccomment_status%2Cping_status%2Cmeta%2Ctemplate%2Cpassword%2Cpermalink_template%2Cgenerated_slug";
    const ALL_SPARSE_PAGE_FIELDS_WITH_EDIT_CONTEXT: &[SparsePageFieldWithEditContext; 24] = &[
        SparsePageFieldWithEditContext::Id,
        SparsePageFieldWithEditContext::Date,
        SparsePageFieldWithEditContext::DateGmt,
        SparsePageFieldWithEditContext::Guid,
        SparsePageFieldWithEditContext::Link,
        SparsePageFieldWithEditContext::Modified,
        SparsePageFieldWithEditContext::ModifiedGmt,
        SparsePageFieldWithEditContext::Slug,
        SparsePageFieldWithEditContext::Status,
        SparsePageFieldWithEditContext::PageType,
        SparsePageFieldWithEditContext::Parent,
        SparsePageFieldWithEditContext::MenuOrder,
        SparsePageFieldWithEditContext::Title,
        SparsePageFieldWithEditContext::Content,
        SparsePageFieldWithEditContext::Author,
        SparsePageFieldWithEditContext::Excerpt,
        SparsePageFieldWithEditContext::FeaturedMedia,
        SparsePageFieldWithEditContext::CommentStatus,
        SparsePageFieldWithEditContext::PingStatus,
        SparsePageFieldWithEditContext::Meta,
        SparsePageFieldWithEditContext::Template,
        SparsePageFieldWithEditContext::Password,
        SparsePageFieldWithEditContext::PermalinkTemplate,
        SparsePageFieldWithEditContext::GeneratedSlug,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_PAGE_FIELDS_WITH_EMBED_CONTEXT: &str =
        "_fields=id%2Cdate%2Clink%2Cslug%2Ctype%2Ctitle%2Cauthor%2Cexcerpt%2Cfeatured_media";
    const ALL_SPARSE_PAGE_FIELDS_WITH_EMBED_CONTEXT: &[SparsePageFieldWithEmbedContext; 9] = &[
        SparsePageFieldWithEmbedContext::Id,
        SparsePageFieldWithEmbedContext::Date,
        SparsePageFieldWithEmbedContext::Link,
        SparsePageFieldWithEmbedContext::Slug,
        SparsePageFieldWithEmbedContext::PageType,
        SparsePageFieldWithEmbedContext::Title,
        SparsePageFieldWithEmbedContext::Author,
        SparsePageFieldWithEmbedContext::Excerpt,
        SparsePageFieldWithEmbedContext::FeaturedMedia,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_PAGE_FIELDS_WITH_VIEW_CONTEXT: &str = "_fields=id%2Cdate%2Cdate_gmt%2Cguid%2Clink%2Cmodified%2Cmodified_gmt%2Cslug%2Cstatus%2Ctype%2Cparent%2Cmenu_order%2Ctitle%2Ccontent%2Cauthor%2Cexcerpt%2Cfeatured_media%2Ccomment_status%2Cping_status%2Cmeta%2Ctemplate";
    const ALL_SPARSE_PAGE_FIELDS_WITH_VIEW_CONTEXT: &[SparsePageFieldWithViewContext; 21] = &[
        SparsePageFieldWithViewContext::Id,
        SparsePageFieldWithViewContext::Date,
        SparsePageFieldWithViewContext::DateGmt,
        SparsePageFieldWithViewContext::Guid,
        SparsePageFieldWithViewContext::Link,
        SparsePageFieldWithViewContext::Modified,
        SparsePageFieldWithViewContext::ModifiedGmt,
        SparsePageFieldWithViewContext::Slug,
        SparsePageFieldWithViewContext::Status,
        SparsePageFieldWithViewContext::PageType,
        SparsePageFieldWithViewContext::Parent,
        SparsePageFieldWithViewContext::MenuOrder,
        SparsePageFieldWithViewContext::Title,
        SparsePageFieldWithViewContext::Content,
        SparsePageFieldWithViewContext::Author,
        SparsePageFieldWithViewContext::Excerpt,
        SparsePageFieldWithViewContext::FeaturedMedia,
        SparsePageFieldWithViewContext::CommentStatus,
        SparsePageFieldWithViewContext::PingStatus,
        SparsePageFieldWithViewContext::Meta,
        SparsePageFieldWithViewContext::Template,
    ];

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> PagesRequestEndpoint {
        PagesRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
