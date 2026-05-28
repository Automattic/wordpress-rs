use wp_derive_request_builder::WpDerivedRequest;

use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        support_tickets::ConversationId,
        unified_conversations::{
            ReplyToUnifiedConversationParams, UnifiedConversation, UnifiedConversationSummary,
        },
    },
};

#[derive(WpDerivedRequest)]
enum UnifiedConversationsRequest {
    #[get(url = "/mobile-support/unified-conversations", output = Vec<UnifiedConversationSummary>)]
    GetUnifiedConversationList,
    #[get(url = "/mobile-support/unified-conversations/<conversation_id>", output = UnifiedConversation)]
    GetUnifiedConversation,
    #[post(url = "/mobile-support/unified-conversations/<conversation_id>", params = &ReplyToUnifiedConversationParams, output = UnifiedConversation)]
    ReplyToUnifiedConversation,
}

impl DerivedRequest for UnifiedConversationsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::V2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::endpoint::tests::{fixture_wp_com_api_url_resolver, validate_wp_com_v2_endpoint},
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn get_unified_conversation_list(endpoint: UnifiedConversationsRequestEndpoint) {
        validate_wp_com_v2_endpoint(
            endpoint.get_unified_conversation_list(),
            "/mobile-support/unified-conversations",
        );
    }

    #[rstest]
    fn get_unified_conversation(endpoint: UnifiedConversationsRequestEndpoint) {
        validate_wp_com_v2_endpoint(
            endpoint.get_unified_conversation(&ConversationId(4396575)),
            "/mobile-support/unified-conversations/4396575",
        );
    }

    #[rstest]
    fn reply_to_unified_conversation(endpoint: UnifiedConversationsRequestEndpoint) {
        validate_wp_com_v2_endpoint(
            endpoint.reply_to_unified_conversation(&ConversationId(4396575)),
            "/mobile-support/unified-conversations/4396575",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> UnifiedConversationsRequestEndpoint {
        UnifiedConversationsRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
