use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        support_tickets::{
            AddMessageToSupportConversationParams, ConversationId, CreateSupportTicketParams,
            SupportConversation, SupportConversationSummary,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SupportTicketsRequest {
    #[post(url = "/mobile-support/conversations", params = &CreateSupportTicketParams, output = SupportConversation)]
    CreateSupportTicket,
    #[get(url = "/mobile-support/conversations", output = Vec<SupportConversationSummary>)]
    GetSupportConversationList,
    #[get(url = "/mobile-support/conversations/<conversation_id>", output = SupportConversation)]
    GetSupportConversation,
    #[post(url = "/mobile-support/conversations/<conversation_id>", params = &AddMessageToSupportConversationParams, output = SupportConversation)]
    AddMessageToSupportConversation,
}

impl DerivedRequest for SupportTicketsRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::V2
    }
}
