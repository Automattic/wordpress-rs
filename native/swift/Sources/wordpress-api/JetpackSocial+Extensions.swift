import WordPressAPIInternal

// MARK: - Read extensions

extension AnyPostWithEditContext {
    public var jetpackSocialPublicizeConnections: [JetpackPublicizeConnection]? {
        additionalFields.flatMap {
            WordPressAPIInternal.jetpackSocialPublicizeConnections(additionalFields: $0)
        }
    }

    public var jetpackSocialPublicizeMessage: String? {
        meta.flatMap {
            WordPressAPIInternal.jetpackSocialPublicizeMessage(meta: $0)
        }
    }

    /// Master toggle for Jetpack Social sharing. Defaults to true on the server.
    public var jetpackSocialPublicizeFeatureEnabled: Bool? {
        meta.flatMap {
            WordPressAPIInternal.jetpackSocialPublicizeFeatureEnabled(meta: $0)
        }
    }

    /// Whether the post has already been shared to all connections. Server-set, read-only.
    public var jetpackSocialPostAlreadyShared: Bool? {
        meta.flatMap {
            WordPressAPIInternal.jetpackSocialPostAlreadyShared(meta: $0)
        }
    }
}

extension AnyPostWithViewContext {
    public var jetpackSocialPublicizeConnections: [JetpackPublicizeConnection]? {
        additionalFields.flatMap {
            WordPressAPIInternal.jetpackSocialPublicizeConnections(additionalFields: $0)
        }
    }

    public var jetpackSocialPublicizeMessage: String? {
        meta.flatMap {
            WordPressAPIInternal.jetpackSocialPublicizeMessage(meta: $0)
        }
    }
}

// MARK: - Write extensions

extension PostCreateParams {
    public mutating func setJetpackSocialPublicizeConnections(
        _ connections: [JetpackPublicizeConnectionUpdate]
    ) {
        self.additionalFields = jetpackSocialSetPublicizeConnections(
            existing: self.additionalFields,
            connections: connections
        )
    }

    public mutating func setJetpackSocialPublicizeMessage(_ message: String) {
        self.meta = jetpackSocialSetPublicizeMessage(
            existing: self.meta,
            message: message
        )
    }

    public mutating func setJetpackSocialPublicizeFeatureEnabled(_ enabled: Bool) {
        self.meta = jetpackSocialSetPublicizeFeatureEnabled(
            existing: self.meta,
            enabled: enabled
        )
    }
}

extension PostUpdateParams {
    public mutating func setJetpackSocialPublicizeConnections(
        _ connections: [JetpackPublicizeConnectionUpdate]
    ) {
        self.additionalFields = jetpackSocialSetPublicizeConnections(
            existing: self.additionalFields,
            connections: connections
        )
    }

    public mutating func setJetpackSocialPublicizeMessage(_ message: String) {
        self.meta = jetpackSocialSetPublicizeMessage(
            existing: self.meta,
            message: message
        )
    }

    public mutating func setJetpackSocialPublicizeFeatureEnabled(_ enabled: Bool) {
        self.meta = jetpackSocialSetPublicizeFeatureEnabled(
            existing: self.meta,
            enabled: enabled
        )
    }
}
