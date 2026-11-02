import Foundation
import WordPressAPIInternal

extension PostWorkingCopyFields {

    /// Make post working copy fields from a post fetched with the edit context.
    public init(remote post: AnyPostWithEditContext) throws {
        self = try postWorkingCopyFieldsFromRemote(post: post)
    }

}

extension PageWorkingCopyFields {

    /// Make page working copy fields from a page fetched with the edit context.
    public init(remote page: AnyPostWithEditContext) throws {
        self = try pageWorkingCopyFieldsFromRemote(page: page)
    }

}

extension RemotePostReference {

    /// The reference to `post` for a working copy made from it, as it was fetched.
    public init(remote post: AnyPostWithEditContext) {
        self = remotePostReferenceFromRemote(post: post)
    }

}
