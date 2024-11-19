use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{
    media::{
        MediaId, MediaListParams, MediaUpdateParams, MediaWithEditContext,
        SparseMediaFieldWithEditContext, SparseMediaFieldWithEmbedContext,
        SparseMediaFieldWithViewContext,
    },
    SparseField,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum MediaRequest {
    #[contextual_paged(url = "/media", params = &MediaListParams, output = Vec<crate::media::SparseMedia>, filter_by = crate::media::SparseMediaField)]
    List,
    #[contextual_get(url = "/media/<media_id>", output = crate::media::SparseMedia, filter_by = crate::media::SparseMediaField)]
    Retrieve,
    #[post(url = "/media", params = &crate::media::MediaCreateParams, output = crate::media::MediaWithEditContext)]
    Create,
    #[delete(url = "/media/<media_id>", output = crate::media::MediaDeleteResponse)]
    Delete,
    #[post(url = "/media/<media_id>", params = &MediaUpdateParams, output = MediaWithEditContext)]
    Update,
}

impl DerivedRequest for MediaRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            // The server always returns an error when `force=false`, so a separate `Trash` action
            // is not implemented.
            MediaRequest::Delete => vec![("force", true.to_string())],
            _ => vec![],
        }
    }

    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

impl SparseField for SparseMediaFieldWithEditContext {
    fn as_str(&self) -> &str {
        match self {
            Self::PostId => "post",
            Self::PostType => "type",
            _ => self.as_field_name(),
        }
    }
}

impl SparseField for SparseMediaFieldWithEmbedContext {
    fn as_str(&self) -> &str {
        match self {
            Self::PostType => "type",
            _ => self.as_field_name(),
        }
    }
}

impl SparseField for SparseMediaFieldWithViewContext {
    fn as_str(&self) -> &str {
        match self {
            Self::PostId => "post",
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
        media::{MediaId, MediaStatus, MediaTypeParam},
        posts::{PostId, WpApiParamPostsOrderBy, WpApiParamPostsSearchColumn},
        request::endpoint::{
            tests::{fixture_api_base_url, validate_wp_v2_endpoint},
            ApiBaseUrl,
        },
        UserId, WpApiParamOrder,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn create_media(endpoint: MediaRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(), "/media");
    }

    #[rstest]
    fn delete_media(endpoint: MediaRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.delete(&MediaId(54)), "/media/54?force=true");
    }

    #[rstest]
    #[case(MediaListParams::default(), "")]
    #[case(generate!(MediaListParams, (page, Some(2))), "page=2")]
    #[case(generate!(MediaListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(MediaListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(MediaListParams, (after, Some("2023-08-14 17:00:00.000".to_string()))), "after=2023-08-14+17%3A00%3A00.000")]
    #[case(generate!(MediaListParams, (modified_after, Some("2023-08-14 17:00:00.000".to_string()))), "modified_after=2023-08-14+17%3A00%3A00.000")]
    #[case(generate!(MediaListParams, (author, vec![UserId(1), UserId(2)])), "author=1%2C2")]
    #[case(generate!(MediaListParams, (author_exclude, vec![UserId(1), UserId(2)])), "author_exclude=1%2C2")]
    #[case(generate!(MediaListParams, (before, Some("2023-08-14 17:00:00.000".to_string()))), "before=2023-08-14+17%3A00%3A00.000")]
    #[case(generate!(MediaListParams, (modified_before, Some("2023-08-14 17:00:00.000".to_string()))), "modified_before=2023-08-14+17%3A00%3A00.000")]
    #[case(generate!(MediaListParams, (exclude, vec![MediaId(1), MediaId(2)])), "exclude=1%2C2")]
    #[case(generate!(MediaListParams, (include, vec![MediaId(1), MediaId(2)])), "include=1%2C2")]
    #[case(generate!(MediaListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(MediaListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(MediaListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), "orderby=author")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Date))), "orderby=date")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Id))), "orderby=id")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Include))), "orderby=include")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Modified))), "orderby=modified")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Parent))), "orderby=parent")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Relevance))), "orderby=relevance")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Title))), "orderby=title")]
    #[case(generate!(MediaListParams, (parent, vec![PostId(44444), PostId(44445)])), "parent=44444%2C44445")]
    #[case(generate!(MediaListParams, (parent_exclude, vec![PostId(55555), PostId(55556)])), "parent_exclude=55555%2C55556")]
    #[case(generate!(MediaListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent])), "search_columns=post_content")]
    #[case(generate!(MediaListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostExcerpt])), "search_columns=post_excerpt")]
    #[case(generate!(MediaListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostTitle])), "search_columns=post_title")]
    #[case(generate!(MediaListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent, WpApiParamPostsSearchColumn::PostExcerpt, WpApiParamPostsSearchColumn::PostTitle])), "search_columns=post_content%2Cpost_excerpt%2Cpost_title")]
    #[case(generate!(MediaListParams, (slug, vec!["foo".to_string(), "bar".to_string()])), "slug=foo%2Cbar")]
    #[case(generate!(MediaListParams, (status, vec![MediaStatus::Inherit])), "status=inherit")]
    #[case(generate!(MediaListParams, (status, vec![MediaStatus::Private])), "status=private")]
    #[case(generate!(MediaListParams, (status, vec![MediaStatus::Trash])), "status=trash")]
    #[case(generate!(MediaListParams, (status, vec![MediaStatus::Custom("foo".to_string())])), "status=foo")]
    #[case(generate!(MediaListParams, (status, vec![MediaStatus::Inherit, MediaStatus::Private, MediaStatus::Trash, MediaStatus::Custom("foo".to_string())])), "status=inherit%2Cprivate%2Ctrash%2Cfoo")]
    #[case(generate!(MediaListParams, (media_type, Some(MediaTypeParam::Image))), "media_type=image")]
    #[case(generate!(MediaListParams, (mime_type, Some("image/jpeg".to_string()))), "mime_type=image%2Fjpeg")]
    #[case(
        media_list_params_with_all_fields(),
        EXPECTED_QUERY_PAIRS_FOR_MEDIA_LIST_PARAMS_WITH_ALL_FIELDS
    )]
    fn list_posts(
        endpoint: MediaRequestEndpoint,
        #[case] params: MediaListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/media?context={}", context)
            } else {
                format!("/media?context={}&{}", context, expected_additional_params)
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
    #[case(MediaListParams::default(), &[], "/media?context=edit&_fields=")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), &[SparseMediaFieldWithEditContext::Author], "/media?context=edit&orderby=author&_fields=author")]
    #[case(media_list_params_with_all_fields(), ALL_SPARSE_MEDIA_FIELDS_WITH_EDIT_CONTEXT, &format!("/media?context=edit&{}&{}", EXPECTED_QUERY_PAIRS_FOR_MEDIA_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_MEDIA_FIELDS_WITH_EDIT_CONTEXT))]
    fn filter_list_post_with_edit_context(
        endpoint: MediaRequestEndpoint,
        #[case] params: MediaListParams,
        #[case] fields: &[SparseMediaFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(MediaListParams::default(), &[], "/media?context=embed&_fields=")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), &[SparseMediaFieldWithEmbedContext::Author], "/media?context=embed&orderby=author&_fields=author")]
    #[case(media_list_params_with_all_fields(), ALL_SPARSE_MEDIA_FIELDS_WITH_EMBED_CONTEXT, &format!("/media?context=embed&{}&{}", EXPECTED_QUERY_PAIRS_FOR_MEDIA_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_MEDIA_FIELDS_WITH_EMBED_CONTEXT))]
    fn filter_list_post_with_embed_context(
        endpoint: MediaRequestEndpoint,
        #[case] params: MediaListParams,
        #[case] fields: &[SparseMediaFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_embed_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(MediaListParams::default(), &[], "/media?context=view&_fields=")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), &[SparseMediaFieldWithViewContext::Author], "/media?context=view&orderby=author&_fields=author")]
    #[case(media_list_params_with_all_fields(), ALL_SPARSE_MEDIA_FIELDS_WITH_VIEW_CONTEXT, &format!("/media?context=view&{}&{}", EXPECTED_QUERY_PAIRS_FOR_MEDIA_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_MEDIA_FIELDS_WITH_VIEW_CONTEXT))]
    fn filter_list_post_with_view_context(
        endpoint: MediaRequestEndpoint,
        #[case] params: MediaListParams,
        #[case] fields: &[SparseMediaFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    fn retrieve_media(endpoint: MediaRequestEndpoint) {
        let media_id = MediaId(77);
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&media_id),
            "/media/77?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&media_id),
            "/media/77?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&media_id),
            "/media/77?context=view",
        );
    }

    #[rstest]
    fn filter_retrieve_media(endpoint: MediaRequestEndpoint) {
        let media_id = MediaId(77);
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_edit_context(
                &media_id,
                &[
                    SparseMediaFieldWithEditContext::Date,
                    SparseMediaFieldWithEditContext::Guid,
                ],
            ),
            "/media/77?context=edit&_fields=date%2Cguid",
        );
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_embed_context(
                &media_id,
                &[
                    SparseMediaFieldWithEmbedContext::Link,
                    SparseMediaFieldWithEmbedContext::PostType,
                ],
            ),
            "/media/77?context=embed&_fields=link%2Ctype",
        );
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(
                &media_id,
                &[
                    SparseMediaFieldWithViewContext::AltText,
                    SparseMediaFieldWithViewContext::Template,
                ],
            ),
            "/media/77?context=view&_fields=alt_text%2Ctemplate",
        );
    }

    const EXPECTED_QUERY_PAIRS_FOR_MEDIA_LIST_PARAMS_WITH_ALL_FIELDS: &str =
        "page=11&per_page=22&search=s_q&after=d_a&modified_after=d_m_a&author=111%2C112&author_exclude=211%2C212&before=d_b&modified_before=d_m_b&exclude=1111%2C1112&include=2111%2C2112&offset=11111&order=desc&orderby=slug&parent=44444%2C44445&search_columns=post_content%2Cpost_excerpt&slug=sl_1%2Csl_2&status=inherit%2Cprivate%2Ctrash&parent_exclude=55555%2C55556&media_type=image&mime_type=image%2Fjpeg";
    fn media_list_params_with_all_fields() -> MediaListParams {
        MediaListParams {
            page: Some(11),
            per_page: Some(22),
            search: Some("s_q".to_string()),
            after: Some("d_a".to_string()),
            modified_after: Some("d_m_a".to_string()),
            author: vec![UserId(111), UserId(112)],
            author_exclude: vec![UserId(211), UserId(212)],
            before: Some("d_b".to_string()),
            modified_before: Some("d_m_b".to_string()),
            exclude: vec![MediaId(1111), MediaId(1112)],
            include: vec![MediaId(2111), MediaId(2112)],
            offset: Some(11111),
            order: Some(WpApiParamOrder::Desc),
            orderby: Some(WpApiParamPostsOrderBy::Slug),
            parent: vec![PostId(44444), PostId(44445)],
            parent_exclude: vec![PostId(55555), PostId(55556)],
            search_columns: vec![
                WpApiParamPostsSearchColumn::PostContent,
                WpApiParamPostsSearchColumn::PostExcerpt,
            ],
            slug: vec!["sl_1".to_string(), "sl_2".to_string()],
            status: vec![
                MediaStatus::Inherit,
                MediaStatus::Private,
                MediaStatus::Trash,
            ],
            media_type: Some(MediaTypeParam::Image),
            mime_type: Some("image/jpeg".to_string()),
        }
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_MEDIA_FIELDS_WITH_EDIT_CONTEXT: &str = "_fields=id%2Cdate%2Cdate_gmt%2Cguid%2Clink%2Cmodified%2Cmodified_gmt%2Cslug%2Cstatus%2Ctype%2Cpassword%2Cpermalink_template%2Cgenerated_slug%2Ctitle%2Cauthor%2Ccomment_status%2Cping_status%2Ctemplate%2Calt_text%2Ccaption%2Cdescription%2Cmedia_type%2Cmime_type%2Cmedia_details%2Cpost%2Csource_url%2Cmissing_image_sizes";
    const ALL_SPARSE_MEDIA_FIELDS_WITH_EDIT_CONTEXT: &[SparseMediaFieldWithEditContext; 27] = &[
        SparseMediaFieldWithEditContext::Id,
        SparseMediaFieldWithEditContext::Date,
        SparseMediaFieldWithEditContext::DateGmt,
        SparseMediaFieldWithEditContext::Guid,
        SparseMediaFieldWithEditContext::Link,
        SparseMediaFieldWithEditContext::Modified,
        SparseMediaFieldWithEditContext::ModifiedGmt,
        SparseMediaFieldWithEditContext::Slug,
        SparseMediaFieldWithEditContext::Status,
        SparseMediaFieldWithEditContext::PostType,
        SparseMediaFieldWithEditContext::Password,
        SparseMediaFieldWithEditContext::PermalinkTemplate,
        SparseMediaFieldWithEditContext::GeneratedSlug,
        SparseMediaFieldWithEditContext::Title,
        SparseMediaFieldWithEditContext::Author,
        SparseMediaFieldWithEditContext::CommentStatus,
        SparseMediaFieldWithEditContext::PingStatus,
        SparseMediaFieldWithEditContext::Template,
        SparseMediaFieldWithEditContext::AltText,
        SparseMediaFieldWithEditContext::Caption,
        SparseMediaFieldWithEditContext::Description,
        SparseMediaFieldWithEditContext::MediaType,
        SparseMediaFieldWithEditContext::MimeType,
        SparseMediaFieldWithEditContext::MediaDetails,
        SparseMediaFieldWithEditContext::PostId,
        SparseMediaFieldWithEditContext::SourceUrl,
        SparseMediaFieldWithEditContext::MissingImageSizes,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_MEDIA_FIELDS_WITH_EMBED_CONTEXT: &str =
        "_fields=id%2Cdate%2Clink%2Cslug%2Ctype%2Ctitle%2Cauthor%2Calt_text%2Ccaption%2Cmedia_type%2Cmime_type%2Cmedia_details%2Csource_url";
    const ALL_SPARSE_MEDIA_FIELDS_WITH_EMBED_CONTEXT: &[SparseMediaFieldWithEmbedContext; 13] = &[
        SparseMediaFieldWithEmbedContext::Id,
        SparseMediaFieldWithEmbedContext::Date,
        SparseMediaFieldWithEmbedContext::Link,
        SparseMediaFieldWithEmbedContext::Slug,
        SparseMediaFieldWithEmbedContext::PostType,
        SparseMediaFieldWithEmbedContext::Title,
        SparseMediaFieldWithEmbedContext::Author,
        SparseMediaFieldWithEmbedContext::AltText,
        SparseMediaFieldWithEmbedContext::Caption,
        SparseMediaFieldWithEmbedContext::MediaType,
        SparseMediaFieldWithEmbedContext::MimeType,
        SparseMediaFieldWithEmbedContext::MediaDetails,
        SparseMediaFieldWithEmbedContext::SourceUrl,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_MEDIA_FIELDS_WITH_VIEW_CONTEXT: &str = "_fields=id%2Cdate%2Cdate_gmt%2Cguid%2Clink%2Cmodified%2Cmodified_gmt%2Cslug%2Cstatus%2Ctype%2Ctitle%2Cauthor%2Ccomment_status%2Cping_status%2Ctemplate%2Calt_text%2Ccaption%2Cdescription%2Cmedia_type%2Cmime_type%2Cmedia_details%2Cpost%2Csource_url";
    const ALL_SPARSE_MEDIA_FIELDS_WITH_VIEW_CONTEXT: &[SparseMediaFieldWithViewContext; 23] = &[
        SparseMediaFieldWithViewContext::Id,
        SparseMediaFieldWithViewContext::Date,
        SparseMediaFieldWithViewContext::DateGmt,
        SparseMediaFieldWithViewContext::Guid,
        SparseMediaFieldWithViewContext::Link,
        SparseMediaFieldWithViewContext::Modified,
        SparseMediaFieldWithViewContext::ModifiedGmt,
        SparseMediaFieldWithViewContext::Slug,
        SparseMediaFieldWithViewContext::Status,
        SparseMediaFieldWithViewContext::PostType,
        SparseMediaFieldWithViewContext::Title,
        SparseMediaFieldWithViewContext::Author,
        SparseMediaFieldWithViewContext::CommentStatus,
        SparseMediaFieldWithViewContext::PingStatus,
        SparseMediaFieldWithViewContext::Template,
        SparseMediaFieldWithViewContext::AltText,
        SparseMediaFieldWithViewContext::Caption,
        SparseMediaFieldWithViewContext::Description,
        SparseMediaFieldWithViewContext::MediaType,
        SparseMediaFieldWithViewContext::MimeType,
        SparseMediaFieldWithViewContext::MediaDetails,
        SparseMediaFieldWithViewContext::PostId,
        SparseMediaFieldWithViewContext::SourceUrl,
    ];

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> MediaRequestEndpoint {
        MediaRequestEndpoint::new(fixture_api_base_url)
    }
}
