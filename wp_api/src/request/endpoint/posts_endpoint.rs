use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::posts::{AnyPostWithEditContext, PostId, PostListParams, PostUpdateParams};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum PostsRequest {
    #[contextual_paged(url = "/<post_endpoint_type>", params = &PostListParams, output = Vec<crate::posts::SparseAnyPost>, filter_by = crate::posts::SparseAnyPostField)]
    List,
    #[contextual_get(url = "/<post_endpoint_type>/<post_id>", params = &crate::posts::PostRetrieveParams, output = crate::posts::SparseAnyPost, filter_by = crate::posts::SparseAnyPostField)]
    Retrieve,
    #[post(url = "/<post_endpoint_type>", params = &crate::posts::PostCreateParams, output = crate::posts::AnyPostWithEditContext)]
    Create,
    #[delete(url = "/<post_endpoint_type>/<post_id>", output = crate::posts::PostDeleteResponse)]
    Delete,
    #[delete(url = "/<post_endpoint_type>/<post_id>", output = crate::posts::AnyPostWithEditContext)]
    Trash,
    #[post(url = "/<post_endpoint_type>/<post_id>", params = &PostUpdateParams, output = AnyPostWithEditContext)]
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
pub enum PostEndpointType {
    Posts,
    Pages,
    Custom(String),
}

impl DerivedRequest for PostsRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            PostsRequest::Delete => vec![("force", true.to_string())],
            PostsRequest::Trash => vec![("force", false.to_string())],
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
        UserId, WpApiParamOrder,
        categories::CategoryId,
        generate,
        posts::{
            PostRetrieveParams, PostStatus, SparseAnyPostFieldWithEditContext,
            SparseAnyPostFieldWithEmbedContext, SparseAnyPostFieldWithViewContext,
            WpApiParamPostsOrderBy, WpApiParamPostsSearchColumn, WpApiParamPostsTaxRelation,
        },
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
        tags::TagId,
        unit_test_common::{
            unit_test_example_date_as_option, unit_test_example_date_as_query_value,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn create_post(endpoint: PostsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(&PostEndpointType::Posts), "/posts");
    }

    #[rstest]
    fn delete_post(endpoint: PostsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.delete(&PostEndpointType::Posts, &PostId(54)),
            "/posts/54?force=true",
        );
    }

    #[rstest]
    #[case(PostListParams::default(), "")]
    #[case(generate!(PostListParams, (page, Some(2))), "page=2")]
    #[case(generate!(PostListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(PostListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(PostListParams, (after, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("after"))]
    #[case(generate!(PostListParams, (modified_after, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("modified_after"))]
    #[case(generate!(PostListParams, (author, vec![UserId(1), UserId(2)])), "author=1%2C2")]
    #[case(generate!(PostListParams, (author_exclude, vec![UserId(1), UserId(2)])), "author_exclude=1%2C2")]
    #[case(generate!(PostListParams, (before, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("before"))]
    #[case(generate!(PostListParams, (modified_before, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("modified_before"))]
    #[case(generate!(PostListParams, (exclude, vec![PostId(1), PostId(2)])), "exclude=1%2C2")]
    #[case(generate!(PostListParams, (include, vec![PostId(1), PostId(2)])), "include=1%2C2")]
    #[case(generate!(PostListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(PostListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(PostListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), "orderby=author")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Date))), "orderby=date")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Id))), "orderby=id")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Include))), "orderby=include")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Modified))), "orderby=modified")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Parent))), "orderby=parent")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Relevance))), "orderby=relevance")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Title))), "orderby=title")]
    #[case(generate!(PostListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent])), "search_columns=post_content")]
    #[case(generate!(PostListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostExcerpt])), "search_columns=post_excerpt")]
    #[case(generate!(PostListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostTitle])), "search_columns=post_title")]
    #[case(generate!(PostListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent, WpApiParamPostsSearchColumn::PostExcerpt, WpApiParamPostsSearchColumn::PostTitle])), "search_columns=post_content%2Cpost_excerpt%2Cpost_title")]
    #[case(generate!(PostListParams, (slug, vec!["foo".to_string(), "bar".to_string()])), "slug=foo%2Cbar")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Draft])), "status=draft")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Future])), "status=future")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Pending])), "status=pending")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Private])), "status=private")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Publish])), "status=publish")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Custom("foo".to_string())])), "status=foo")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Draft, PostStatus::Future, PostStatus::Pending, PostStatus::Private, PostStatus::Publish, PostStatus::Custom("foo".to_string())])), "status=draft%2Cfuture%2Cpending%2Cprivate%2Cpublish%2Cfoo")]
    #[case(generate!(PostListParams, (tax_relation, Some(WpApiParamPostsTaxRelation::And))), "tax_relation=AND")]
    #[case(generate!(PostListParams, (tax_relation, Some(WpApiParamPostsTaxRelation::Or))), "tax_relation=OR")]
    #[case(generate!(PostListParams, (categories, vec![CategoryId(1), CategoryId(2)])), "categories=1%2C2")]
    #[case(generate!(PostListParams, (categories_exclude, vec![CategoryId(1), CategoryId(2)])), "categories_exclude=1%2C2")]
    #[case(generate!(PostListParams, (tags, vec![TagId(1), TagId(2)])), "tags=1%2C2")]
    #[case(generate!(PostListParams, (tags_exclude, vec![TagId(1), TagId(2)])), "tags_exclude=1%2C2")]
    #[case(generate!(PostListParams, (sticky, Some(true))), "sticky=true")]
    #[case(
        post_list_params_with_all_fields(),
        &expected_query_pairs_for_post_list_params_with_all_fields()
    )]
    fn list_posts(
        endpoint: PostsRequestEndpoint,
        #[case] params: PostListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/posts?context={context}")
            } else {
                format!("/posts?context={context}&{expected_additional_params}")
            }
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&PostEndpointType::Posts, &params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&PostEndpointType::Posts, &params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&PostEndpointType::Posts, &params),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(PostListParams::default(), &[], "/posts?context=edit&_fields=")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), &[SparseAnyPostFieldWithEditContext::Author], "/posts?context=edit&orderby=author&_fields=author")]
    #[case(post_list_params_with_all_fields(), ALL_SPARSE_POST_FIELDS_WITH_EDIT_CONTEXT, &format!("/posts?context=edit&{}&{}", expected_query_pairs_for_post_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_FIELDS_WITH_EDIT_CONTEXT))]
    fn filter_list_post_with_edit_context(
        endpoint: PostsRequestEndpoint,
        #[case] params: PostListParams,
        #[case] fields: &[SparseAnyPostFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&PostEndpointType::Posts, &params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(PostListParams::default(), &[], "/posts?context=embed&_fields=")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), &[SparseAnyPostFieldWithEmbedContext::Author], "/posts?context=embed&orderby=author&_fields=author")]
    #[case(post_list_params_with_all_fields(), ALL_SPARSE_POST_FIELDS_WITH_EMBED_CONTEXT, &format!("/posts?context=embed&{}&{}", expected_query_pairs_for_post_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_FIELDS_WITH_EMBED_CONTEXT))]
    fn filter_list_post_with_embed_context(
        endpoint: PostsRequestEndpoint,
        #[case] params: PostListParams,
        #[case] fields: &[SparseAnyPostFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_embed_context(&PostEndpointType::Posts, &params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(None, "")]
    #[case(Some("foo"), "password=foo")]
    fn retrieve_post(
        endpoint: PostsRequestEndpoint,
        #[case] password: Option<&str>,
        #[case] expected_additional_params: &str,
    ) {
        let post_id = PostId(54);
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/posts/54?context={context}")
            } else {
                format!("/posts/54?context={context}&{expected_additional_params}")
            }
        };
        let params = PostRetrieveParams {
            password: password.map(|p| p.to_string()),
        };
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&PostEndpointType::Posts, &post_id, &params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&PostEndpointType::Posts, &post_id, &params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&PostEndpointType::Posts, &post_id, &params),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(None, &[], "/posts/54?context=view&_fields=")]
    #[case(Some("foo"), &[SparseAnyPostFieldWithViewContext::Author], "/posts/54?context=view&password=foo&_fields=author")]
    #[case(Some("foo"), ALL_SPARSE_POST_FIELDS_WITH_VIEW_CONTEXT, &format!("/posts/54?context=view&password=foo&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_FIELDS_WITH_VIEW_CONTEXT}"))]
    fn filter_retrieve_post_with_view_context(
        endpoint: PostsRequestEndpoint,
        #[case] password: Option<&str>,
        #[case] fields: &[SparseAnyPostFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(
                &PostEndpointType::Posts,
                &PostId(54),
                &PostRetrieveParams {
                    password: password.map(|p| p.to_string()),
                },
                fields,
            ),
            expected_path,
        );
    }

    #[rstest]
    fn trash_post(endpoint: PostsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.trash(&PostEndpointType::Posts, &PostId(54)),
            "/posts/54?force=false",
        );
    }

    #[rstest]
    fn update_post(endpoint: PostsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.update(&PostEndpointType::Posts, &PostId(54)),
            "/posts/54",
        );
    }

    fn expected_query_pairs_for_post_list_params_with_all_fields() -> String {
        let after = unit_test_example_date_as_query_value("after");
        let modified_after = unit_test_example_date_as_query_value("modified_after");
        let before = unit_test_example_date_as_query_value("before");
        let modified_before = unit_test_example_date_as_query_value("modified_before");
        format!(
            "page=2&per_page=2&search=foo&{after}&{modified_after}&author=1%2C2&author_exclude=1%2C2&{before}&{modified_before}&exclude=1%2C2&include=1%2C2&offset=2&order=asc&orderby=author&search_columns=post_content%2Cpost_excerpt%2Cpost_title&slug=foo%2Cbar&status=draft%2Cfuture%2Cpending%2Cprivate%2Cpublish%2Cfoo&tax_relation=AND&categories=1%2C2&categories_exclude=1%2C2&tags=1%2C2&tags_exclude=1%2C2&sticky=true&parent=1&parent_exclude=1%2C2&menu_order=1"
        )
    }

    fn post_list_params_with_all_fields() -> PostListParams {
        PostListParams {
            after: unit_test_example_date_as_option(),
            author: vec![UserId(1), UserId(2)],
            author_exclude: vec![UserId(1), UserId(2)],
            before: unit_test_example_date_as_option(),
            categories: vec![CategoryId(1), CategoryId(2)],
            categories_exclude: vec![CategoryId(1), CategoryId(2)],
            exclude: vec![PostId(1), PostId(2)],
            include: vec![PostId(1), PostId(2)],
            modified_after: unit_test_example_date_as_option(),
            modified_before: unit_test_example_date_as_option(),
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
            status: vec![
                PostStatus::Draft,
                PostStatus::Future,
                PostStatus::Pending,
                PostStatus::Private,
                PostStatus::Publish,
                PostStatus::Custom("foo".to_string()),
            ],
            sticky: Some(true),
            tags: vec![TagId(1), TagId(2)],
            tags_exclude: vec![TagId(1), TagId(2)],
            tax_relation: Some(WpApiParamPostsTaxRelation::And),
            parent: Some(PostId(1)),
            parent_exclude: vec![PostId(1), PostId(2)],
            menu_order: Some(1),
        }
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_FIELDS_WITH_EDIT_CONTEXT: &str = "_fields=id%2Cdate%2Cdate_gmt%2Cguid%2Clink%2Cmodified%2Cmodified_gmt%2Cslug%2Cstatus%2Ctitle%2Ccontent%2Cauthor%2Cexcerpt%2Cfeatured_media%2Ccomment_status%2Cping_status%2Cformat%2Cmeta%2Csticky%2Ctemplate%2Ccategories%2Ctags%2Cparent%2Cmenu_order%2Cpassword%2Cpermalink_template%2Cgenerated_slug";
    const ALL_SPARSE_POST_FIELDS_WITH_EDIT_CONTEXT: &[SparseAnyPostFieldWithEditContext; 27] = &[
        SparseAnyPostFieldWithEditContext::Id,
        SparseAnyPostFieldWithEditContext::Date,
        SparseAnyPostFieldWithEditContext::DateGmt,
        SparseAnyPostFieldWithEditContext::Guid,
        SparseAnyPostFieldWithEditContext::Link,
        SparseAnyPostFieldWithEditContext::Modified,
        SparseAnyPostFieldWithEditContext::ModifiedGmt,
        SparseAnyPostFieldWithEditContext::Slug,
        SparseAnyPostFieldWithEditContext::Status,
        SparseAnyPostFieldWithEditContext::Title,
        SparseAnyPostFieldWithEditContext::Content,
        SparseAnyPostFieldWithEditContext::Author,
        SparseAnyPostFieldWithEditContext::Excerpt,
        SparseAnyPostFieldWithEditContext::FeaturedMedia,
        SparseAnyPostFieldWithEditContext::CommentStatus,
        SparseAnyPostFieldWithEditContext::PingStatus,
        SparseAnyPostFieldWithEditContext::Format,
        SparseAnyPostFieldWithEditContext::Meta,
        SparseAnyPostFieldWithEditContext::Sticky,
        SparseAnyPostFieldWithEditContext::Template,
        SparseAnyPostFieldWithEditContext::Categories,
        SparseAnyPostFieldWithEditContext::Tags,
        SparseAnyPostFieldWithEditContext::Parent,
        SparseAnyPostFieldWithEditContext::MenuOrder,
        SparseAnyPostFieldWithEditContext::Password,
        SparseAnyPostFieldWithEditContext::PermalinkTemplate,
        SparseAnyPostFieldWithEditContext::GeneratedSlug,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_FIELDS_WITH_EMBED_CONTEXT: &str =
        "_fields=id%2Clink%2Cslug%2Ctitle%2Cauthor%2Cexcerpt%2Cfeatured_media";
    const ALL_SPARSE_POST_FIELDS_WITH_EMBED_CONTEXT: &[SparseAnyPostFieldWithEmbedContext; 7] = &[
        SparseAnyPostFieldWithEmbedContext::Id,
        SparseAnyPostFieldWithEmbedContext::Link,
        SparseAnyPostFieldWithEmbedContext::Slug,
        SparseAnyPostFieldWithEmbedContext::Title,
        SparseAnyPostFieldWithEmbedContext::Author,
        SparseAnyPostFieldWithEmbedContext::Excerpt,
        SparseAnyPostFieldWithEmbedContext::FeaturedMedia,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_FIELDS_WITH_VIEW_CONTEXT: &str = "_fields=id%2Cdate%2Cdate_gmt%2Cguid%2Clink%2Cmodified%2Cmodified_gmt%2Cslug%2Cstatus%2Ctitle%2Ccontent%2Cauthor%2Cexcerpt%2Cfeatured_media%2Ccomment_status%2Cping_status%2Cformat%2Cmeta%2Csticky%2Ctemplate%2Ccategories%2Ctags%2Cparent%2Cmenu_order";
    const ALL_SPARSE_POST_FIELDS_WITH_VIEW_CONTEXT: &[SparseAnyPostFieldWithViewContext; 24] = &[
        SparseAnyPostFieldWithViewContext::Id,
        SparseAnyPostFieldWithViewContext::Date,
        SparseAnyPostFieldWithViewContext::DateGmt,
        SparseAnyPostFieldWithViewContext::Guid,
        SparseAnyPostFieldWithViewContext::Link,
        SparseAnyPostFieldWithViewContext::Modified,
        SparseAnyPostFieldWithViewContext::ModifiedGmt,
        SparseAnyPostFieldWithViewContext::Slug,
        SparseAnyPostFieldWithViewContext::Status,
        SparseAnyPostFieldWithViewContext::Title,
        SparseAnyPostFieldWithViewContext::Content,
        SparseAnyPostFieldWithViewContext::Author,
        SparseAnyPostFieldWithViewContext::Excerpt,
        SparseAnyPostFieldWithViewContext::FeaturedMedia,
        SparseAnyPostFieldWithViewContext::CommentStatus,
        SparseAnyPostFieldWithViewContext::PingStatus,
        SparseAnyPostFieldWithViewContext::Format,
        SparseAnyPostFieldWithViewContext::Meta,
        SparseAnyPostFieldWithViewContext::Sticky,
        SparseAnyPostFieldWithViewContext::Template,
        SparseAnyPostFieldWithViewContext::Categories,
        SparseAnyPostFieldWithViewContext::Tags,
        SparseAnyPostFieldWithViewContext::Parent,
        SparseAnyPostFieldWithViewContext::MenuOrder,
    ];

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> PostsRequestEndpoint {
        PostsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
