import Foundation
import Testing
import WordPressAPI

final class LocalContentFromRemoteTests {
    private let directory: URL
    private let site = SiteIdentity(url: TestCredentials.instance().siteURL, wordpressComSiteId: nil)

    init() throws {
        directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("LocalContentFromRemoteTests-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    }

    deinit {
        try? FileManager.default.removeItem(at: directory)
    }

    private func openService() throws -> LocalContentService {
        let store = try WpLocalContentStore(path: directory.appendingPathComponent("local_content.db").path)
        _ = try store.performMigrations()
        return LocalContentService(store: store)
    }

    private func fetchFirst(_ endpoint: PostEndpointType) async throws -> AnyPostWithEditContext {
        let posts = try await WordPressAPI.admin().posts
            .listWithEditContext(postEndpointType: endpoint, params: PostListParams())
            .data
        return try #require(posts.first)
    }

    @Test func workingCopyFromAFetchedPostPointsAtIt() async throws {
        let post = try await fetchFirst(.posts)
        let service = try openService()

        let target = WorkingCopyRemoteTarget(site: site, remotePost: RemotePostReference(remote: post))
        let created = try service.createPostWorkingCopy(
            remoteTarget: target,
            fields: PostWorkingCopyFields(remote: post)
        )

        #expect(created.fields.common.title == post.title?.raw)
        #expect(created.fields.common.content == post.content.raw)
        #expect(
            created.remoteTarget.remotePost == RemotePostReference(postId: post.id, baseModifiedGmt: post.modifiedGmt)
        )
    }

    @Test func postWorkingCopyFromAFetchedPageThrows() async throws {
        let page = try await fetchFirst(.pages)

        #expect(throws: RemotePostConversionError.WrongPostType(expected: "post", actual: "page")) {
            try PostWorkingCopyFields(remote: page)
        }
    }
}
