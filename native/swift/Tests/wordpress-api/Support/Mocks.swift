import Foundation
import WordPressAPI

final class MockAppNotifier: WpAppNotifier {
    func requestedWithInvalidAuthentication(requestUrl: String) async {
        // no-op
    }
}
