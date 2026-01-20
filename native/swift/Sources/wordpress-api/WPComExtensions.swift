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

public extension SupportAttachment {
    var dimensions: AttachmentDimensions? {
        getAttachmentDimensions(attachment: self)
    }
}

public extension StatsVisitsResponse {
    var statsVisitsData: [StatsVisitsDataPoint] {
        getStatsVisitsData(response: self)
    }

    var statsVisitorsData: [StatsVisitorsDataPoint] {
        getStatsVisitorsData(response: self)
    }

    var statsLikesData: [StatsLikesDataPoint] {
        getStatsLikesData(response: self)
    }

    var statsReblogsData: [StatsReblogsDataPoint] {
        getStatsReblogsData(response: self)
    }

    var statsCommentsData: [StatsCommentsDataPoint] {
        getStatsCommentsData(response: self)
    }

    var statsPostsData: [StatsPostsDataPoint] {
        getStatsPostsData(response: self)
    }
}
