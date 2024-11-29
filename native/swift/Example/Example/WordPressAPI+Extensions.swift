import Foundation
import WordPressAPI

extension WordPressAPI {

    static var globalInstance: WordPressAPI {
        get async throws {
            let loginManager = await LoginManager()

            guard let defaultSiteUrl = await loginManager.getDefaultSiteUrl() else {
                throw CocoaError(.validationMissingMandatoryProperty)
            }

            let parsedUrl = try ParsedUrl.parse(input: defaultSiteUrl)

            guard let loginCredentials = try await loginManager.getLoginCredentials() else {
                throw CocoaError(.xpcConnectionInvalid)
            }

            return WordPressAPI(
               urlSession: .shared,
               baseUrl: parsedUrl,
               authenticationStategy: loginCredentials
           )
        }
    }
}
