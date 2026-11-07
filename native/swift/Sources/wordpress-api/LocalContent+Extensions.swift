import Foundation
import WordPressAPIInternal

extension PostWorkingCopyFields {

    /// Convert a post fetched with the edit context into working copy fields.
    ///
    /// Throws `RemotePostConversionError.WrongPostType` if `post` isn't a post,
    /// `.UneditableStatus` if it's in the trash, and `.MissingField` if it
    /// wasn't fetched with the edit context.
    public init(remote post: AnyPostWithEditContext) throws {
        self = try postWorkingCopyFieldsFromRemote(post: post)
    }

}

extension PageWorkingCopyFields {

    /// Convert a page fetched with the edit context into working copy fields.
    ///
    /// Throws `RemotePostConversionError.WrongPostType` if `page` isn't a page,
    /// `.UneditableStatus` if it's in the trash, and `.MissingField` if it
    /// wasn't fetched with the edit context.
    public init(remote page: AnyPostWithEditContext) throws {
        self = try pageWorkingCopyFieldsFromRemote(page: page)
    }

}

extension RemotePostReference {

    /// Reference `post`, with its `modified_gmt` as the base version.
    ///
    /// Use it with the fetched post a working copy is created from, and with
    /// the post the server returns after a push.
    public init(remote post: AnyPostWithEditContext) {
        self = remotePostReferenceFromRemote(post: post)
    }

}
