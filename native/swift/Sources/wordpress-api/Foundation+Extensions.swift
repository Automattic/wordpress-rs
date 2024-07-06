import Foundation

public extension Date {

    private static let wordpressDateFormatter: DateFormatter = {
        let dateFormatter = DateFormatter()
        dateFormatter.locale = Locale(identifier: "en_US_POSIX")
        dateFormatter.timeZone = TimeZone(abbreviation: "GMT")
        dateFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss"

        return dateFormatter
    }()

    /// Parses a date string provided by WordPress APIs (which are assumed to be in GMT)
    ///
    static func fromWordPressDate(_ string: String) -> Date? {
        wordpressDateFormatter.date(from: string)
    }
}
