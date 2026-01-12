import Foundation
import WordPressAPI

extension WordPressAPI {

    static var globalInstance: WordPressAPI {
        get async throws {
            guard let defaultSiteUrl = await LoginManager.shared.getApiRootUrl() else {
                throw CocoaError(.validationMissingMandatoryProperty)
            }

            let apiRootUrl = try ParsedUrl.parse(input: defaultSiteUrl)

            guard let loginCredentials = try await LoginManager.shared.getLoginCredentials() else {
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
}

extension WPComApiClient {
    static var globalInstance: WPComApiClient {
        get async throws {
            guard let loginCredentials = try await LoginManager.shared.getWpComLoginCredentials() else {
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
}
