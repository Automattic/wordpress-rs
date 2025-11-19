import WordPressAPI

final class AppNotifier: WpAppNotifier {
    func requestedWithInvalidAuthentication(requestUrl: String) async {
        // Ignore this for tests
    }
}
