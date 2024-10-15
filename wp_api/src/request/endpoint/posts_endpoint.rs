use crate::{
    posts::{
        PostId, PostListParams, PostUpdateParams, PostWithEditContext,
        SparsePostFieldWithEditContext, SparsePostFieldWithEmbedContext,
        SparsePostFieldWithViewContext,
    },
    SparseField,
};
use wp_derive_request_builder::WpDerivedRequest;

use super::{AsNamespace, DerivedRequest, WpNamespace};

#[derive(WpDerivedRequest)]
enum PostsRequest {
    #[contextual_get(url = "/posts", params = &PostListParams, output = Vec<crate::posts::SparsePost>, filter_by = crate::posts::SparsePostField)]
    List,
    #[contextual_get(url = "/posts/<post_id>", params = &crate::posts::PostRetrieveParams, output = crate::posts::SparsePost, filter_by = crate::posts::SparsePostField)]
    Retrieve,
    #[post(url = "/posts", params = &crate::posts::PostCreateParams, output = crate::posts::PostWithEditContext)]
    Create,
    #[delete(url = "/posts/<post_id>", output = crate::posts::PostDeleteResponse)]
    Delete,
    #[delete(url = "/posts/<post_id>", output = crate::posts::PostWithEditContext)]
    Trash,
    #[post(url = "/posts/<post_id>", params = &PostUpdateParams, output = PostWithEditContext)]
    Update,
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

impl SparseField for SparsePostFieldWithEditContext {
    fn as_str(&self) -> &str {
        match self {
            Self::PostType => "type",
            _ => self.as_field_name(),
        }
    }
}

impl SparseField for SparsePostFieldWithEmbedContext {
    fn as_str(&self) -> &str {
        match self {
            Self::PostType => "type",
            _ => self.as_field_name(),
        }
    }
}

impl SparseField for SparsePostFieldWithViewContext {
    fn as_str(&self) -> &str {
        match self {
            Self::PostType => "type",
            _ => self.as_field_name(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        generate,
        posts::{
            CategoryId, PostRetrieveParams, PostStatus, TagId, WpApiParamPostsOrderBy,
            WpApiParamPostsSearchColumn, WpApiParamPostsTaxRelation,
        },
        request::endpoint::{
            tests::{fixture_api_base_url, validate_wp_v2_endpoint},
            ApiBaseUrl,
        },
        UserId, WpApiParamOrder,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn create_post(endpoint: PostsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(), "/posts");
    }

    #[rstest]
    fn delete_post(endpoint: PostsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.delete(&PostId(54)), "/posts/54?force=true");
    }

    #[rstest]
    #[case(PostListParams::default(), "")]
    #[case(generate!(PostListParams, (page, Some(2))), "page=2")]
    #[case(generate!(PostListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(PostListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(PostListParams, (after, Some("2023-08-14 17:00:00.000".to_string()))), "after=2023-08-14+17%3A00%3A00.000")]
    #[case(generate!(PostListParams, (modified_after, Some("2023-08-14 17:00:00.000".to_string()))), "modified_after=2023-08-14+17%3A00%3A00.000")]
    #[case(generate!(PostListParams, (author, vec![UserId(1), UserId(2)])), "author=1%2C2")]
    #[case(generate!(PostListParams, (author_exclude, vec![UserId(1), UserId(2)])), "author_exclude=1%2C2")]
    #[case(generate!(PostListParams, (before, Some("2023-08-14 17:00:00.000".to_string()))), "before=2023-08-14+17%3A00%3A00.000")]
    #[case(generate!(PostListParams, (modified_before, Some("2023-08-14 17:00:00.000".to_string()))), "modified_before=2023-08-14+17%3A00%3A00.000")]
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
        EXPECTED_QUERY_PAIRS_FOR_POST_LIST_PARAMS_WITH_ALL_FIELDS
    )]
    fn list_posts(
        endpoint: PostsRequestEndpoint,
        #[case] params: PostListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/posts?context={}", context)
            } else {
                format!("/posts?context={}&{}", context, expected_additional_params)
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
    #[case(PostListParams::default(), &[], "/posts?context=edit&_fields=")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), &[SparsePostFieldWithEditContext::Author], "/posts?context=edit&orderby=author&_fields=author")]
    #[case(post_list_params_with_all_fields(), ALL_SPARSE_POST_FIELDS_WITH_EDIT_CONTEXT, &format!("/posts?context=edit&{}&{}", EXPECTED_QUERY_PAIRS_FOR_POST_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_FIELDS_WITH_EDIT_CONTEXT))]
    fn filter_list_post_with_edit_context(
        endpoint: PostsRequestEndpoint,
        #[case] params: PostListParams,
        #[case] fields: &[SparsePostFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(PostListParams::default(), &[], "/posts?context=embed&_fields=")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), &[SparsePostFieldWithEmbedContext::Author], "/posts?context=embed&orderby=author&_fields=author")]
    #[case(post_list_params_with_all_fields(), ALL_SPARSE_POST_FIELDS_WITH_EMBED_CONTEXT, &format!("/posts?context=embed&{}&{}", EXPECTED_QUERY_PAIRS_FOR_POST_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_FIELDS_WITH_EMBED_CONTEXT))]
    fn filter_list_post_with_embed_context(
        endpoint: PostsRequestEndpoint,
        #[case] params: PostListParams,
        #[case] fields: &[SparsePostFieldWithEmbedContext],
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
    fn retrieve_post(
        endpoint: PostsRequestEndpoint,
        #[case] password: Option<&str>,
        #[case] expected_additional_params: &str,
    ) {
        let post_id = PostId(54);
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/posts/54?context={}", context)
            } else {
                format!(
                    "/posts/54?context={}&{}",
                    context, expected_additional_params
                )
            }
        };
        let params = PostRetrieveParams {
            password: password.map(|p| p.to_string()),
        };
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&post_id, &params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&post_id, &params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&post_id, &params),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(None, &[], "/posts/54?context=view&_fields=")]
    #[case(Some("foo"), &[SparsePostFieldWithViewContext::Author], "/posts/54?context=view&password=foo&_fields=author")]
    #[case(Some("foo"), ALL_SPARSE_POST_FIELDS_WITH_VIEW_CONTEXT, &format!("/posts/54?context=view&password=foo&{}", EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_FIELDS_WITH_VIEW_CONTEXT))]
    fn filter_retrieve_post_with_view_context(
        endpoint: PostsRequestEndpoint,
        #[case] password: Option<&str>,
        #[case] fields: &[SparsePostFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(
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
        validate_wp_v2_endpoint(endpoint.trash(&PostId(54)), "/posts/54?force=false");
    }

    #[rstest]
    fn update_post(endpoint: PostsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.update(&PostId(54)), "/posts/54");
    }

    const EXPECTED_QUERY_PAIRS_FOR_POST_LIST_PARAMS_WITH_ALL_FIELDS: &str =
        "page=2&per_page=2&search=foo&after=2023-08-14+17%3A00%3A00.000&modified_after=2023-08-14+17%3A00%3A00.000&author=1%2C2&author_exclude=1%2C2&before=2023-08-14+17%3A00%3A00.000&modified_before=2023-08-14+17%3A00%3A00.000&exclude=1%2C2&include=1%2C2&offset=2&order=asc&orderby=author&search_columns=post_content%2Cpost_excerpt%2Cpost_title&slug=foo%2Cbar&status=draft%2Cfuture%2Cpending%2Cprivate%2Cpublish%2Cfoo&tax_relation=AND&categories=1%2C2&categories_exclude=1%2C2&tags=1%2C2&tags_exclude=1%2C2&sticky=true";
    fn post_list_params_with_all_fields() -> PostListParams {
        PostListParams {
            after: Some("2023-08-14 17:00:00.000".to_string()),
            author: vec![UserId(1), UserId(2)],
            author_exclude: vec![UserId(1), UserId(2)],
            before: Some("2023-08-14 17:00:00.000".to_string()),
            categories: vec![CategoryId(1), CategoryId(2)],
            categories_exclude: vec![CategoryId(1), CategoryId(2)],
            exclude: vec![PostId(1), PostId(2)],
            include: vec![PostId(1), PostId(2)],
            modified_after: Some("2023-08-14 17:00:00.000".to_string()),
            modified_before: Some("2023-08-14 17:00:00.000".to_string()),
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
        }
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_FIELDS_WITH_EDIT_CONTEXT: &str = "_fields=id%2Cdate%2Cdate_gmt%2Cguid%2Clink%2Cmodified%2Cmodified_gmt%2Cslug%2Cstatus%2Ctitle%2Ccontent%2Cauthor%2Cexcerpt%2Cfeatured_media%2Ccomment_status%2Cping_status%2Cformat%2Cmeta%2Csticky%2Ctemplate%2Ccategories%2Ctags%2Cpassword%2Cpermalink_template%2Cgenerated_slug";
    const ALL_SPARSE_POST_FIELDS_WITH_EDIT_CONTEXT: &[SparsePostFieldWithEditContext; 25] = &[
        SparsePostFieldWithEditContext::Id,
        SparsePostFieldWithEditContext::Date,
        SparsePostFieldWithEditContext::DateGmt,
        SparsePostFieldWithEditContext::Guid,
        SparsePostFieldWithEditContext::Link,
        SparsePostFieldWithEditContext::Modified,
        SparsePostFieldWithEditContext::ModifiedGmt,
        SparsePostFieldWithEditContext::Slug,
        SparsePostFieldWithEditContext::Status,
        SparsePostFieldWithEditContext::Title,
        SparsePostFieldWithEditContext::Content,
        SparsePostFieldWithEditContext::Author,
        SparsePostFieldWithEditContext::Excerpt,
        SparsePostFieldWithEditContext::FeaturedMedia,
        SparsePostFieldWithEditContext::CommentStatus,
        SparsePostFieldWithEditContext::PingStatus,
        SparsePostFieldWithEditContext::Format,
        SparsePostFieldWithEditContext::Meta,
        SparsePostFieldWithEditContext::Sticky,
        SparsePostFieldWithEditContext::Template,
        SparsePostFieldWithEditContext::Categories,
        SparsePostFieldWithEditContext::Tags,
        SparsePostFieldWithEditContext::Password,
        SparsePostFieldWithEditContext::PermalinkTemplate,
        SparsePostFieldWithEditContext::GeneratedSlug,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_FIELDS_WITH_EMBED_CONTEXT: &str =
        "_fields=id%2Clink%2Cslug%2Ctitle%2Cauthor%2Cexcerpt%2Cfeatured_media";
    const ALL_SPARSE_POST_FIELDS_WITH_EMBED_CONTEXT: &[SparsePostFieldWithEmbedContext; 7] = &[
        SparsePostFieldWithEmbedContext::Id,
        SparsePostFieldWithEmbedContext::Link,
        SparsePostFieldWithEmbedContext::Slug,
        SparsePostFieldWithEmbedContext::Title,
        SparsePostFieldWithEmbedContext::Author,
        SparsePostFieldWithEmbedContext::Excerpt,
        SparsePostFieldWithEmbedContext::FeaturedMedia,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_POST_FIELDS_WITH_VIEW_CONTEXT: &str = "_fields=id%2Cdate%2Cdate_gmt%2Cguid%2Clink%2Cmodified%2Cmodified_gmt%2Cslug%2Cstatus%2Ctitle%2Ccontent%2Cauthor%2Cexcerpt%2Cfeatured_media%2Ccomment_status%2Cping_status%2Cformat%2Cmeta%2Csticky%2Ctemplate%2Ccategories%2Ctags";
    const ALL_SPARSE_POST_FIELDS_WITH_VIEW_CONTEXT: &[SparsePostFieldWithViewContext; 22] = &[
        SparsePostFieldWithViewContext::Id,
        SparsePostFieldWithViewContext::Date,
        SparsePostFieldWithViewContext::DateGmt,
        SparsePostFieldWithViewContext::Guid,
        SparsePostFieldWithViewContext::Link,
        SparsePostFieldWithViewContext::Modified,
        SparsePostFieldWithViewContext::ModifiedGmt,
        SparsePostFieldWithViewContext::Slug,
        SparsePostFieldWithViewContext::Status,
        SparsePostFieldWithViewContext::Title,
        SparsePostFieldWithViewContext::Content,
        SparsePostFieldWithViewContext::Author,
        SparsePostFieldWithViewContext::Excerpt,
        SparsePostFieldWithViewContext::FeaturedMedia,
        SparsePostFieldWithViewContext::CommentStatus,
        SparsePostFieldWithViewContext::PingStatus,
        SparsePostFieldWithViewContext::Format,
        SparsePostFieldWithViewContext::Meta,
        SparsePostFieldWithViewContext::Sticky,
        SparsePostFieldWithViewContext::Template,
        SparsePostFieldWithViewContext::Categories,
        SparsePostFieldWithViewContext::Tags,
    ];

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> PostsRequestEndpoint {
        PostsRequestEndpoint::new(fixture_api_base_url)
    }
}
