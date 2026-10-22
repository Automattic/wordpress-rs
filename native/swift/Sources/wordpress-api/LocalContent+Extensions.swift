import Foundation
import WordPressAPIInternal

extension PostWorkingCopy {

    /// Make a post working copy from a post fetched with the edit context.
    public init(remote post: AnyPostWithEditContext) throws {
        self = try postWorkingCopyFromRemote(post: post)
    }

}

extension PageWorkingCopy {

    /// Make a page working copy from a page fetched with the edit context.
    public init(remote page: AnyPostWithEditContext) throws {
        self = try pageWorkingCopyFromRemote(page: page)
    }

}

extension RemotePostReference {

    /// The reference to `post` for a working copy made from it, as it was fetched.
    public init(remote post: AnyPostWithEditContext) {
        self = remotePostReferenceFromRemote(post: post)
    }

}
