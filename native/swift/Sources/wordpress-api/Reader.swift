import Foundation
import WordPressAPIInternal

extension ReaderLikesResponse {
    /// The parameters for the page after this one, asking for `number` likes, or `nil` when this is the last
    /// page.
    public func nextPageParams(number: UInt32) -> ReaderLikesParams? {
        readerLikesNextPageParams(response: self, number: number)
    }
}
