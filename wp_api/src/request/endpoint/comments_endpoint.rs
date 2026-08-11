use crate::comments::{CommentId, CommentListParams, CommentUpdateParams};
use wp_derive_request_builder::WpDerivedRequest;

use super::{AsNamespace, DerivedRequest, WpNamespace};
#[derive(WpDerivedRequest)]
enum CommentsRequest {
    #[contextual_paged(url = "/comments", params = &CommentListParams, output = Vec<crate::comments::SparseComment>, filter_by = crate::comments::SparseCommentField)]
    List,
    #[contextual_get(url = "/comments/<comment_id>", params = &crate::comments::CommentRetrieveParams, output = crate::comments::SparseComment, filter_by = crate::comments::SparseCommentField)]
    Retrieve,
    #[post(url = "/comments", params = &crate::comments::CommentCreateParams, output = crate::comments::CommentWithEditContext)]
    Create,
    #[delete(url = "/comments/<comment_id>", params = &crate::comments::CommentDeleteParams, output = crate::comments::CommentDeleteResponse)]
    Delete,
    #[delete(url = "/comments/<comment_id>", params = &crate::comments::CommentDeleteParams, output = crate::comments::CommentWithEditContext)]
    Trash,
    #[post(url = "/comments/<comment_id>", params = &CommentUpdateParams, output = crate::comments::CommentWithEditContext)]
    Update,
}

impl DerivedRequest for CommentsRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            CommentsRequest::Delete => vec![("force", true.to_string())],
            CommentsRequest::Trash => vec![("force", false.to_string())],
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
        UserId, WpApiParamOrder,
        comments::{
            CommentDeleteParams, CommentId, CommentRetrieveParams, CommentStatus, CommentType,
            SparseCommentFieldWithEditContext, SparseCommentFieldWithEmbedContext,
            SparseCommentFieldWithViewContext, WpApiParamCommentsOrderBy,
        },
        generate,
        posts::PostId,
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
    #[case(CommentListParams::default(), "")]
    #[case(generate!(CommentListParams, (page, Some(2))), "page=2")]
    #[case(generate!(CommentListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(CommentListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(CommentListParams, (after, unit_test_example_date_string_as_option())), &unit_test_example_date_string_as_query_value("after"))]
    #[case(generate!(CommentListParams, (author, vec![UserId(1), UserId(2)])), "author=1%2C2")]
    #[case(generate!(CommentListParams, (author_exclude, vec![UserId(1), UserId(2)])), "author_exclude=1%2C2")]
    #[case(generate!(CommentListParams, (author_email, Some("foo".to_string()))), "author_email=foo")]
    #[case(generate!(CommentListParams, (before, unit_test_example_date_string_as_option())), &unit_test_example_date_string_as_query_value("before"))]
    #[case(generate!(CommentListParams, (exclude, vec![CommentId(1), CommentId(2)])), "exclude=1%2C2")]
    #[case(generate!(CommentListParams, (include, vec![CommentId(1), CommentId(2)])), "include=1%2C2")]
    #[case(generate!(CommentListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(CommentListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(CommentListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Date))), "orderby=date")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::DateGmt))), "orderby=date_gmt")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Id))), "orderby=id")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Include))), "orderby=include")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Post))), "orderby=post")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Parent))), "orderby=parent")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Type))), "orderby=type")]
    #[case(generate!(CommentListParams, (parent, vec![CommentId(44444), CommentId(44445)])), "parent=44444%2C44445")]
    #[case(generate!(CommentListParams, (parent_exclude, vec![CommentId(55555), CommentId(55556)])), "parent_exclude=55555%2C55556")]
    #[case(generate!(CommentListParams, (post, vec![PostId(66666), PostId(66667)])), "post=66666%2C66667")]
    #[case(generate!(CommentListParams, (status, Some(CommentStatus::Hold))), "status=hold")]
    #[case(generate!(CommentListParams, (status, Some(CommentStatus::Approved))), "status=approved")]
    #[case(generate!(CommentListParams, (status, Some(CommentStatus::Spam))), "status=spam")]
    #[case(generate!(CommentListParams, (status, Some(CommentStatus::Trash))), "status=trash")]
    #[case(generate!(CommentListParams, (status, Some(CommentStatus::Custom("foo".to_string())))), "status=foo")]
    #[case(generate!(CommentListParams, (comment_type, Some(CommentType::Comment))), "type=comment")]
    #[case(generate!(CommentListParams, (comment_type, Some(CommentType::Pingback))), "type=pingback")]
    #[case(generate!(CommentListParams, (comment_type, Some(CommentType::Trackback))), "type=trackback")]
    #[case(generate!(CommentListParams, (comment_type, Some(CommentType::Custom("foo".to_string())))), "type=foo")]
    #[case(generate!(CommentListParams, (password, Some("foo".to_string()))), "password=foo")]
    #[case(
        comment_list_params_with_all_fields(),
        &expected_query_pairs_for_comment_list_params_with_all_fields()
    )]
    fn list_comments(
        endpoint: CommentsRequestEndpoint,
        #[case] params: CommentListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/comments?context={context}")
            } else {
                format!("/comments?context={context}&{expected_additional_params}")
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
    #[case(CommentListParams::default(), &[], "/comments?context=edit&_fields=")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Id))), &[SparseCommentFieldWithEditContext::Author], "/comments?context=edit&orderby=id&_fields=author")]
    #[case(comment_list_params_with_all_fields(), ALL_SPARSE_COMMENT_FIELDS_WITH_EDIT_CONTEXT, &format!("/comments?context=edit&{}&{}", expected_query_pairs_for_comment_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_COMMENT_FIELDS_WITH_EDIT_CONTEXT))]
    fn filter_list_comments_with_edit_context(
        endpoint: CommentsRequestEndpoint,
        #[case] params: CommentListParams,
        #[case] fields: &[SparseCommentFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(CommentListParams::default(), &[], "/comments?context=embed&_fields=")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::DateGmt))), &[SparseCommentFieldWithEmbedContext::Author], "/comments?context=embed&orderby=date_gmt&_fields=author")]
    #[case(comment_list_params_with_all_fields(), ALL_SPARSE_COMMENT_FIELDS_WITH_EMBED_CONTEXT, &format!("/comments?context=embed&{}&{}", expected_query_pairs_for_comment_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_COMMENT_FIELDS_WITH_EMBED_CONTEXT))]
    fn filter_list_comments_with_embed_context(
        endpoint: CommentsRequestEndpoint,
        #[case] params: CommentListParams,
        #[case] fields: &[SparseCommentFieldWithEmbedContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_embed_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(CommentListParams::default(), &[], "/comments?context=view&_fields=")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Include))), &[SparseCommentFieldWithViewContext::Author], "/comments?context=view&orderby=include&_fields=author")]
    #[case(comment_list_params_with_all_fields(), ALL_SPARSE_COMMENT_FIELDS_WITH_VIEW_CONTEXT, &format!("/comments?context=view&{}&{}", expected_query_pairs_for_comment_list_params_with_all_fields(), EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_COMMENT_FIELDS_WITH_VIEW_CONTEXT))]
    fn filter_list_comments_with_view_context(
        endpoint: CommentsRequestEndpoint,
        #[case] params: CommentListParams,
        #[case] fields: &[SparseCommentFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&params, fields),
            expected_path,
        );
    }

    fn expected_query_pairs_for_comment_list_params_with_all_fields() -> String {
        let after = unit_test_example_date_string_as_query_value("after");
        let before = unit_test_example_date_string_as_query_value("before");
        format!(
            "page=11&per_page=22&search=s_q&{after}&author=111%2C112&author_exclude=211%2C212&author_email=a_email%40example.com&{before}&exclude=1111%2C1112&include=2111%2C2112&offset=11111&order=desc&orderby=type&parent=44444%2C44445&parent_exclude=55555%2C55556&post=66666%2C66667&status=spam&type=pingback&password=p_q"
        )
    }

    fn comment_list_params_with_all_fields() -> CommentListParams {
        CommentListParams {
            page: Some(11),
            per_page: Some(22),
            search: Some("s_q".to_string()),
            after: unit_test_example_date_string_as_option(),
            author: vec![UserId(111), UserId(112)],
            author_exclude: vec![UserId(211), UserId(212)],
            author_email: Some("a_email@example.com".to_string()),
            before: unit_test_example_date_string_as_option(),
            exclude: vec![CommentId(1111), CommentId(1112)],
            include: vec![CommentId(2111), CommentId(2112)],
            offset: Some(11111),
            order: Some(WpApiParamOrder::Desc),
            orderby: Some(WpApiParamCommentsOrderBy::Type),
            parent: vec![CommentId(44444), CommentId(44445)],
            parent_exclude: vec![CommentId(55555), CommentId(55556)],
            post: vec![PostId(66666), PostId(66667)],
            status: Some(CommentStatus::Spam),
            comment_type: Some(CommentType::Pingback),
            password: Some("p_q".to_string()),
        }
    }

    #[rstest]
    #[case(None, "")]
    #[case(Some("foo"), "password=foo")]
    fn retrieve_comment(
        endpoint: CommentsRequestEndpoint,
        #[case] password: Option<&str>,
        #[case] expected_additional_params: &str,
    ) {
        let comment_id = CommentId(54);
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/comments/54?context={context}")
            } else {
                format!("/comments/54?context={context}&{expected_additional_params}")
            }
        };
        let params = CommentRetrieveParams {
            password: password.map(|p| p.to_string()),
        };
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&comment_id, &params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&comment_id, &params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&comment_id, &params),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(None, &[], "/comments/54?context=view&_fields=")]
    #[case(Some("foo"), &[SparseCommentFieldWithViewContext::Author], "/comments/54?context=view&password=foo&_fields=author")]
    #[case(Some("foo"), ALL_SPARSE_COMMENT_FIELDS_WITH_VIEW_CONTEXT, &format!("/comments/54?context=view&password=foo&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_COMMENT_FIELDS_WITH_VIEW_CONTEXT}"))]
    fn filter_retrieve_comment_with_view_context(
        endpoint: CommentsRequestEndpoint,
        #[case] password: Option<&str>,
        #[case] fields: &[SparseCommentFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(
                &CommentId(54),
                &CommentRetrieveParams {
                    password: password.map(|p| p.to_string()),
                },
                fields,
            ),
            expected_path,
        );
    }

    #[rstest]
    fn create_comment(endpoint: CommentsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(), "/comments");
    }

    #[rstest]
    #[case(None, "/comments/54?force=true")]
    #[case(Some("foo".to_string()), "/comments/54?password=foo&force=true")]
    fn delete_comment(
        endpoint: CommentsRequestEndpoint,
        #[case] password: Option<String>,
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.delete(&CommentId(54), &CommentDeleteParams::new(password)),
            expected_path,
        );
    }

    #[rstest]
    #[case(None, "/comments/54?force=false")]
    #[case(Some("foo".to_string()), "/comments/54?password=foo&force=false")]
    fn trash_comment(
        endpoint: CommentsRequestEndpoint,
        #[case] password: Option<String>,
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.trash(&CommentId(54), &CommentDeleteParams::new(password)),
            expected_path,
        );
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_COMMENT_FIELDS_WITH_EDIT_CONTEXT: &str = "_fields=id%2Cauthor%2Cauthor_email%2Cauthor_ip%2Cauthor_name%2Cauthor_url%2Cauthor_user_agent%2Ccontent%2Cdate%2Cdate_gmt%2Clink%2Cparent%2Cpost%2Cstatus%2Ctype%2Cauthor_avatar_urls";
    const ALL_SPARSE_COMMENT_FIELDS_WITH_EDIT_CONTEXT: &[SparseCommentFieldWithEditContext; 16] = &[
        SparseCommentFieldWithEditContext::Id,
        SparseCommentFieldWithEditContext::Author,
        SparseCommentFieldWithEditContext::AuthorEmail,
        SparseCommentFieldWithEditContext::AuthorIp,
        SparseCommentFieldWithEditContext::AuthorName,
        SparseCommentFieldWithEditContext::AuthorUrl,
        SparseCommentFieldWithEditContext::AuthorUserAgent,
        SparseCommentFieldWithEditContext::Content,
        SparseCommentFieldWithEditContext::Date,
        SparseCommentFieldWithEditContext::DateGmt,
        SparseCommentFieldWithEditContext::Link,
        SparseCommentFieldWithEditContext::Parent,
        SparseCommentFieldWithEditContext::Post,
        SparseCommentFieldWithEditContext::Status,
        SparseCommentFieldWithEditContext::CommentType,
        SparseCommentFieldWithEditContext::AuthorAvatarUrls,
    ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_COMMENT_FIELDS_WITH_EMBED_CONTEXT: &str = "_fields=id%2Cauthor%2Cauthor_name%2Cauthor_url%2Ccontent%2Cdate%2Clink%2Cparent%2Ctype%2Cauthor_avatar_urls";
    const ALL_SPARSE_COMMENT_FIELDS_WITH_EMBED_CONTEXT: &[SparseCommentFieldWithEmbedContext; 10] =
        &[
            SparseCommentFieldWithEmbedContext::Id,
            SparseCommentFieldWithEmbedContext::Author,
            SparseCommentFieldWithEmbedContext::AuthorName,
            SparseCommentFieldWithEmbedContext::AuthorUrl,
            SparseCommentFieldWithEmbedContext::Content,
            SparseCommentFieldWithEmbedContext::Date,
            SparseCommentFieldWithEmbedContext::Link,
            SparseCommentFieldWithEmbedContext::Parent,
            SparseCommentFieldWithEmbedContext::CommentType,
            SparseCommentFieldWithEmbedContext::AuthorAvatarUrls,
        ];

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_COMMENT_FIELDS_WITH_VIEW_CONTEXT: &str = "_fields=id%2Cauthor%2Cauthor_name%2Cauthor_url%2Ccontent%2Cdate%2Cdate_gmt%2Clink%2Cparent%2Cpost%2Cstatus%2Ctype%2Cauthor_avatar_urls";
    const ALL_SPARSE_COMMENT_FIELDS_WITH_VIEW_CONTEXT: &[SparseCommentFieldWithViewContext; 13] = &[
        SparseCommentFieldWithViewContext::Id,
        SparseCommentFieldWithViewContext::Author,
        SparseCommentFieldWithViewContext::AuthorName,
        SparseCommentFieldWithViewContext::AuthorUrl,
        SparseCommentFieldWithViewContext::Content,
        SparseCommentFieldWithViewContext::Date,
        SparseCommentFieldWithViewContext::DateGmt,
        SparseCommentFieldWithViewContext::Link,
        SparseCommentFieldWithViewContext::Parent,
        SparseCommentFieldWithViewContext::Post,
        SparseCommentFieldWithViewContext::Status,
        SparseCommentFieldWithViewContext::CommentType,
        SparseCommentFieldWithViewContext::AuthorAvatarUrls,
    ];

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> CommentsRequestEndpoint {
        CommentsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
