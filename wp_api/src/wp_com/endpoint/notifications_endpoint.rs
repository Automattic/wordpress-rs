use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        notifications::{
            NOTE_HASH_FIELDS, NOTE_ID_FIELDS, NotificationHashesResponse, NotificationId,
            NotificationIdsResponse, NotificationParams, NotificationsParams, NotificationsQuery,
            NotificationsResponse,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum NotificationsRequest {
    // A page of the user's notes, newest first, whole.
    #[get(url = "/notifications", params = &NotificationsParams, output = NotificationsResponse)]
    List,
    // A page of the user's notes as IDs and hashes, to find the ones that changed.
    #[get(url = "/notifications", params = &NotificationsQuery, output = NotificationHashesResponse)]
    ListHashes,
    // A page of the user's notes as IDs only.
    #[get(url = "/notifications", params = &NotificationsQuery, output = NotificationIdsResponse)]
    ListIds,
    // One note. One that doesn't exist is a success with no notes, not a 404.
    #[get(url = "/notifications/<notification_id>", params = &NotificationParams, output = NotificationsResponse)]
    Get,
}

impl DerivedRequest for NotificationsRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        // Whole notes name their fields in their params, which say whether to send the body
        // and the note context.
        match self {
            NotificationsRequest::ListHashes => vec![("fields", NOTE_HASH_FIELDS.to_string())],
            NotificationsRequest::ListIds => vec![("fields", NOTE_ID_FIELDS.to_string())],
            NotificationsRequest::List | NotificationsRequest::Get => Vec::new(),
        }
    }

    fn namespace(&self) -> impl AsNamespace {
        // `rest/v1.1` is the version the apps and the web panel use.
        WpComNamespace::RestV1_1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::{
            endpoint::tests::{
                fixture_wp_com_api_url_resolver, language_provider,
                validate_wp_com_rest_v1_1_endpoint,
            },
            language::WPComLanguage,
            notifications::NotificationType,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    const WHOLE_NOTE_FIELDS: &str = "fields=id%2Ctype%2Csubtype%2Cvariant%2Cread%2Ctimestamp%2Cicon%2Cnoticon%2Ctitle%2Curl%2Csubject%2Cmeta%2Cnote_hash";

    fn every_filter() -> NotificationsQuery {
        NotificationsQuery {
            number: Some(200),
            before: Some("2026-03-04T05:06:07Z".parse().unwrap()),
            since: Some("2026-01-02T00:00:00Z".parse().unwrap()),
            ids: vec![
                NotificationId(10_000_000_001),
                NotificationId(10_000_000_002),
            ],
            types: vec![
                NotificationType::Like,
                NotificationType::CommentLike,
                NotificationType::Other("example_note".to_string()),
            ],
            unread: true,
            read: true,
            locale: Some(WPComLanguage::German),
            source: Some("example-app".to_string()),
        }
    }

    const EVERY_FILTER: &str = "number=200&before=1772600767&since=1767312000&ids=10000000001%2C10000000002&type=like%2Ccomment_like%2Cexample_note&unread=1&read=1&locale=de&source=example-app";

    #[rstest]
    #[case::default(NotificationsParams::default(), format!("/notifications?{WHOLE_NOTE_FIELDS}%2Cheader%2Cbody"))]
    #[case::without_body(
        NotificationsParams { include_body: false, ..Default::default() },
        format!("/notifications?{WHOLE_NOTE_FIELDS}")
    )]
    #[case::with_context(
        NotificationsParams { include_context: true, ..Default::default() },
        format!("/notifications?{WHOLE_NOTE_FIELDS}%2Cheader%2Cbody%2Cnote_context")
    )]
    #[case::without_body_with_context(
        NotificationsParams { include_body: false, include_context: true, ..Default::default() },
        format!("/notifications?{WHOLE_NOTE_FIELDS}%2Cnote_context")
    )]
    #[case::every_filter(
        NotificationsParams { query: every_filter(), ..Default::default() },
        format!("/notifications?{EVERY_FILTER}&{WHOLE_NOTE_FIELDS}%2Cheader%2Cbody")
    )]
    fn list(
        endpoint: NotificationsRequestEndpoint,
        #[case] params: NotificationsParams,
        #[case] expected_path: String,
    ) {
        validate_wp_com_rest_v1_1_endpoint(endpoint.list(&params), &expected_path);
    }

    #[rstest]
    #[case::default(NotificationsQuery::default(), "/notifications?fields=id%2Cnote_hash".to_string())]
    #[case::every_filter(every_filter(), format!("/notifications?{EVERY_FILTER}&fields=id%2Cnote_hash"))]
    fn list_hashes(
        endpoint: NotificationsRequestEndpoint,
        #[case] params: NotificationsQuery,
        #[case] expected_path: String,
    ) {
        validate_wp_com_rest_v1_1_endpoint(endpoint.list_hashes(&params), &expected_path);
    }

    #[rstest]
    #[case::default(NotificationsQuery::default(), "/notifications?fields=id".to_string())]
    #[case::every_filter(every_filter(), format!("/notifications?{EVERY_FILTER}&fields=id"))]
    fn list_ids(
        endpoint: NotificationsRequestEndpoint,
        #[case] params: NotificationsQuery,
        #[case] expected_path: String,
    ) {
        validate_wp_com_rest_v1_1_endpoint(endpoint.list_ids(&params), &expected_path);
    }

    #[rstest]
    #[case::default(
        NotificationParams::default(),
        format!("/notifications/10000000001?{WHOLE_NOTE_FIELDS}%2Cheader%2Cbody")
    )]
    #[case::every_param(
        NotificationParams {
            include_body: false,
            include_context: true,
            locale: Some(WPComLanguage::PortugueseBrazil),
            source: Some("example-app".to_string()),
        },
        format!("/notifications/10000000001?locale=pt-br&source=example-app&{WHOLE_NOTE_FIELDS}%2Cnote_context")
    )]
    fn get(
        endpoint: NotificationsRequestEndpoint,
        #[case] params: NotificationParams,
        #[case] expected_path: String,
    ) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.get(&NotificationId(10_000_000_001), &params),
            &expected_path,
        );
    }

    /// The client's language goes first, so a `locale` in the params, which comes later, is the
    /// one the API reads.
    #[rstest]
    fn locale_from_the_client(fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>) {
        let endpoint = NotificationsRequestEndpoint::with_language_provider(
            fixture_wp_com_api_url_resolver,
            language_provider(Some(WPComLanguage::Spanish)),
        );
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.list_ids(&NotificationsQuery::default()),
            "/notifications?locale=es&fields=id",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> NotificationsRequestEndpoint {
        NotificationsRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
