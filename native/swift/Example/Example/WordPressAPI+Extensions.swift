import Foundation
import WordPressAPI

extension WordPressAPI {
    static var globalInstance: WordPressAPI {
        get throws {
            let loginManager = LoginManager()

            return try WordPressAPI(
               urlSession: .shared,
               baseUrl: URL(string: loginManager.getDefaultSiteUrl()!)!,
               authenticationStategy: loginManager.getLoginCredentials()!
           )
        }
    }

}
