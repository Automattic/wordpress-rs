import Foundation

public extension Date {

    private static let wordpressDateFormatter: DateFormatter = {
        let dateFormatter = DateFormatter()
        dateFormatter.locale = Locale(identifier: "en_US_POSIX")
        dateFormatter.timeZone = TimeZone(abbreviation: "GMT")
        dateFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss"

        return dateFormatter
    }()

    /// Parses an offsetless WordPress datetime string as GMT — the shape of
    /// WordPress's `_gmt` fields.
    ///
    /// Those fields cross the bindings as `Date`, so this is for a string
    /// obtained some other way. It is the wrong tool for `date` and
    /// `modified`, which are in the site's timezone rather than GMT: reading
    /// them with this shifts the instant by the site's UTC offset.
    static func fromWordPressDate(_ string: String) -> Date? {
        wordpressDateFormatter.date(from: string)
    }
}
