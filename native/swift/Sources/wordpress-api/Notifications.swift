import Foundation
import WordPressAPIInternal

extension NotificationText {
    /// The line's text as runs to show, in order. Their `text` put together is the whole line.
    public var styledRuns: [StyledTextRun] {
        notificationTextStyledRuns(text: self)
    }
}

extension NotificationBlock {
    /// The block's text as runs to show, in order. Their `text` put together is the whole block.
    public var styledRuns: [StyledTextRun] {
        notificationBlockStyledRuns(block: self)
    }
}

extension WpComNotification {
    /// How to show the note, in one of five layouts, as the web notifications panel does.
    /// A note fetched without its note context is always a notice.
    public var layout: NotificationLayout {
        notificationLayout(note: self)
    }
}

extension NotificationLabel {
    /// The label in the user's preferred language, or in English until it's translated.
    public var localizedText: String {
        localizedText(languages: Locale.preferredLanguages)
    }

    /// The label in the first of `languages` that has translations, or in English.
    public func localizedText(languages: [String]) -> String {
        localizeNotificationLabel(value: self, locale: wpLocaleResolve(langIds: languages))
    }
}

extension NotificationTab {
    /// The tab's name in the user's preferred language, or in English until it's translated.
    public var localizedText: String {
        localizedText(languages: Locale.preferredLanguages)
    }

    /// The tab's name in the first of `languages` that has translations, or in English.
    public func localizedText(languages: [String]) -> String {
        localizeNotificationTab(value: self, locale: wpLocaleResolve(langIds: languages))
    }
}
