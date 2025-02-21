import WordPressAPIInternal

// This file defines "stapling" extensions – anywhere we want to "staple" a free-floating function to an object

extension UniffiWpLoginClient {
    static func withConfig(
        requestExecutor: any RequestExecutor,
        config: WpLoginClientConfiguration
    ) -> UniffiWpLoginClient {
        return uniffiwploginclientWithConfig(requestExecutor: requestExecutor, config: config)
    }
}

extension WpLoginClientConfiguration {
    public static var `default`: WpLoginClientConfiguration {
        defaultWpLoginClientConfiguration()
    }
}
