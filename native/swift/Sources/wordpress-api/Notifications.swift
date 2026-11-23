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
