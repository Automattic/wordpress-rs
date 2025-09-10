import Foundation
import WordPressAPIInternal

class EmptyAppNotifier: @unchecked Sendable, WpAppNotifier {
    func requestedWithInvalidAuthentication(requestUrl: String) async {
        // no-op
    }
}
