import WordPressAPIInternal

public extension BotMessageContext {
    var userWantsToTalkToAHuman: Bool {
        WordPressAPIInternal.userWantsToTalkToAHuman(context: self)
    }
}

public extension BotConversation {
    var userWantsToTalkToAHuman: Bool {
        for message in self.messages {
            if case .bot(let botContext) = message.context {
                if botContext.userWantsToTalkToAHuman {
                    return true
                }
            }
        }

        return false
    }
}

