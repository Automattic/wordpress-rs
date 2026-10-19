import Foundation
import Testing
import WordPressAPI

final class LocalContentFromRemoteTests {
    private let directory: URL
    private let site = SiteIdentity.selfHosted(url: TestCredentials.instance().siteUrl)

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

        let id = try service.createPostWorkingCopyFromRemote(site: site, post: post)
        let read = try #require(try service.readPostWorkingCopy(id: id))

        #expect(read.workingCopy.common.title == post.title?.raw)
        #expect(read.workingCopy.common.content == post.content.raw)
        #expect(read.target.remotePost == RemotePostReference(postId: post.id, baseModifiedGmt: post.modifiedGmt))
    }

    @Test func workingCopyFromAPageFetchedAsAPostThrows() async throws {
        let page = try await fetchFirst(.pages)
        let service = try openService()

        do {
            _ = try service.createPostWorkingCopyFromRemote(site: site, post: page)
            Issue.record("Creating a post working copy from a page should throw")
        } catch LocalContentError.RemotePostConversion(let error) {
            #expect(error == .wrongPostType(expected: "post", actual: "page"))
        }
    }
}
