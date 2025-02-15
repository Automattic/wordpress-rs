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

public extension TimeInterval {
    static func fromRetryHeaderValue(_ value: String, now: Date = Date()) -> TimeInterval? {
        if let doubleValue = Double(value) {
            return doubleValue
        }

        // Try parsing as a HTTP date format
        let dateFormatter = DateFormatter()
        dateFormatter.locale = Locale(identifier: "en_US_POSIX")
        dateFormatter.timeZone = TimeZone(abbreviation: "GMT")
        dateFormatter.dateFormat = "E, dd MMM yyyy HH:mm:ss zzz"

        if let date = dateFormatter.date(from: value) {
            return date.timeIntervalSince(now)
        }

        return nil
    }
}

extension HTTPURLResponse {
    /// How long until we should retry the request?
    var retryAfter: TimeInterval? {
        guard let stringValue = value(forHTTPHeaderField: "Retry-After") else {
            return nil
        }

        return .fromRetryHeaderValue(stringValue)
    }
}
