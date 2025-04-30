import Foundation
import WordPressAPIInternal

class AppNotifier: @unchecked Sendable, WpAppNotifier {
    weak var api: WordPressAPI?

    func requestedWithInvalidAuthentication() async {
        NotificationCenter.default.post(name: WordPressAPI.requestedWithInvalidAuthenticationNotification, object: api)
    }
}
