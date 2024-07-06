import Foundation
import WordPressAPI

extension WordPressAPI {
    static var globalInstance: WordPressAPI {
        get throws {
            let loginManager = LoginManager()

            let parsedUrl = try WpParsedUrl.parse(input: loginManager.getDefaultSiteUrl()!)

            return try WordPressAPI(
               urlSession: .shared,
               baseUrl: parsedUrl,
               authenticationStategy: loginManager.getLoginCredentials()!
           )
        }
    }

}
