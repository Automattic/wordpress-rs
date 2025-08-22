import Foundation
import WordPressAPIInternal

public class WPComApiClient {
    private let internalClient: WordPressAPIInternal.UniffiWpComApiClient
    private let delegate: WpApiClientDelegate

    public init(delegate: WpApiClientDelegate) {
        self.delegate = delegate // We need to retain this ourselves because it's passed to a Rust object
        self.internalClient = UniffiWpComApiClient(delegate: delegate)
    }

    public var oauth2: Oauth2RequestExecutor {
        internalClient.oauth2()
    }

    public var jetpackConnection: JetpackConnectionRequestExecutor {
        internalClient.jetpackConnection()
    }

    public var subscribers: SubscribersRequestExecutor {
        internalClient.subscribers()
    }

    public var supportBots: SupportBotsRequestExecutor {
        internalClient.supportBots()
    }

    public var supportEligibility: SupportEligibilityRequestExecutor {
        internalClient.supportEligibility()
    }

    public var supportTickets: SupportTicketsRequestExecutor {
        internalClient.supportTickets()
    }
}
