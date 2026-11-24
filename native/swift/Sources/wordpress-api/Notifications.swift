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

extension NotificationServiceError {
    /// A message to show the user for the error, in their preferred language, or in English until
    /// it's translated. `nil` for an error only a developer should see.
    public var userMessage: String? {
        userMessage(languages: Locale.preferredLanguages)
    }

    /// A message to show the user in the first of `languages` that has translations, or `nil`.
    public func userMessage(languages: [String]) -> String? {
        notificationServiceErrorUserMessage(error: self, locale: wpLocaleResolve(langIds: languages))
    }
}
