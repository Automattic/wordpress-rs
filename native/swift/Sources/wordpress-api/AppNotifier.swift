import Foundation
import WordPressAPIInternal

class EmptyAppNotifier: @unchecked Sendable, WpAppNotifier {
    func requestedWithInvalidAuthentication() async {
        // no-op
    }
}
