import Foundation
import WordPressAPI

extension WordPressAPI {

    static func instance(loginManager: LoginManager) async throws -> WordPressAPI {

        guard let defaultSiteUrl = try await loginManager.getApiRootUrl() else {
            throw CocoaError(.validationMissingMandatoryProperty)
        }

        let apiRootUrl = try ParsedUrl.parse(input: defaultSiteUrl)

        guard let loginCredentials = try await loginManager.getLoginCredentials() else {
            throw CocoaError(.xpcConnectionInvalid)
        }

        return WordPressAPI(
           urlSession: .shared,
           apiRootUrl: apiRootUrl,
           authentication: loginCredentials,
           middlewarePipeline: MiddlewarePipeline(middlewares: [
                DebugMiddleware()
           ])
       )
    }

}

extension WPComApiClient {
    static func instance(loginManager: LoginManager) async throws -> WPComApiClient {
        guard let loginCredentials = try await loginManager.getWpComLoginCredentials() else {
            preconditionFailure("Don't access `globalInstance` unless you're logged into WP.com")
        }

        return WPComApiClient(
            authentication: loginCredentials,
            middlewarePipeline: MiddlewarePipeline(middlewares: [
                DebugMiddleware()
            ])
        )
    }
}
