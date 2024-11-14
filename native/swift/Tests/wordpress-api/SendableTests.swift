import Testing
import WordPressAPI

struct SendableTests {

    private static let sendables: [Sendable] = [
        MediaRequestListWithEditContextResponse.empty,
        MediaRequestListWithViewContextResponse.empty,
        MediaRequestListWithEmbedContextResponse.empty,

        PostsRequestListWithEditContextResponse.empty,
        PostsRequestListWithViewContextResponse.empty,
        PostsRequestListWithEmbedContextResponse.empty,

        UsersRequestListWithEditContextResponse.empty,
        UsersRequestListWithViewContextResponse.empty,
        UsersRequestListWithEmbedContextResponse.empty
    ]

    /// This might seem like a weird test – why are we checking such a specific implementation detail?
    ///
    /// This ensures that we don't inadvertently change these types (which we are adding an unsafe `Sendable`
    /// conformance to) in Uniffi from `uniffi::Record` to `uniffi::Object`. This removes their ability to
    /// be `Sendable`, and our conformance would no longer be safe
    @Test("Test that late-conforming sendable types are safe", arguments: sendables)
    func testThatTypesAreSendable(_ type: Sendable) {
        #expect(Mirror(reflecting: type).displayStyle == .struct)
    }
}
