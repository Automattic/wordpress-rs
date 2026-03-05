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

extension WpComSiteIdentifier: ExpressibleByStringLiteral, ExpressibleByIntegerLiteral {
    public init(stringLiteral value: StringLiteralType) {
        self = .slug(value: value)
    }

    public init(integerLiteral value: IntegerLiteralType) {
        precondition(value > 0, "WpComSiteIdentifier must be a positive integer")
        self = .id(value: UInt64(value))
    }

    public init?(_ value: IntegerLiteralType?) {
        guard let value else {
            return nil
        }

        self = WpComSiteIdentifier(integerLiteral: value)
    }
}
