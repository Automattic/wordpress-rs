use crate::wp_com::support_bots::BotId;
use crate::wp_com::support_bots::ChatId;
use crate::wp_com::support_bots::ListBotConversationsParams;
use crate::wp_com::support_bots::MessageId;
use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        support_bots::{
            AddMessageToBotConversationParams, BotConversation, BotConversationSummary,
            CreateBotConversationFeedbackParams, CreateBotConversationParams,
            GetBotConversationParams,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SupportBotsRequest {
    #[post(url = "/odie/chat/<bot_id>", params = &CreateBotConversationParams, output = BotConversation)]
    CreateBotConversation,
    #[get(url = "/odie/conversations/<bot_id>", params = &ListBotConversationsParams, output = Vec<BotConversationSummary>)]
    GetBotConversationList,
    #[get(url = "/odie/chat/<bot_id>/<chat_id>", params = &GetBotConversationParams, output = BotConversation)]
    GetBotConversation,
    #[post(url = "/odie/chat/<bot_id>/<chat_id>", params = &AddMessageToBotConversationParams, output = BotConversation)]
    AddMessageToBotConversation,
    #[post(url = "/odie/chat/<bot_id>/<chat_id>/<message_id>/feedback", params = &CreateBotConversationFeedbackParams, output = bool)]
    CreateBotConversationFeedback,
}

impl DerivedRequest for SupportBotsRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::V2
    }
}
