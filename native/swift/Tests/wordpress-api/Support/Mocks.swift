import Foundation
import WordPressAPI

final class MockAppNotifier: WpAppNotifier {
    func requestedWithInvalidAuthentication() async {
        // no-op
    }
}
