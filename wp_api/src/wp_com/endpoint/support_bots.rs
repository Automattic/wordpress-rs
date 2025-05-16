use crate::{
    impl_as_query_value_for_new_type,
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
use serde::{Deserialize, Serialize};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SupportBotsRequest {
    #[post(url = "/odie/chat/<bot_id>", params = &CreateBotConversationParams, output = BotConversation)]
    CreateBotConversation,

    #[get(url = "/odie/conversations/<bot_id>", output = Vec<BotConversationSummary>)]
    GetBotConverationList,

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

impl_as_query_value_for_new_type!(BotId);
uniffi::custom_newtype!(BotId, String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BotId(pub String);

impl std::fmt::Display for BotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl_as_query_value_for_new_type!(ChatId);
uniffi::custom_newtype!(ChatId, u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatId(pub u64);

impl std::str::FromStr for ChatId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl std::fmt::Display for ChatId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl_as_query_value_for_new_type!(MessageId);
uniffi::custom_newtype!(MessageId, u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageId(pub u64);

impl std::str::FromStr for MessageId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
