import Foundation
import Testing
import WordPressAPI

final class LocalContentTests {
    private let directory: URL

    private var databasePath: String {
        directory.appendingPathComponent("local_content.db").path
    }

    private let siteUrl: ParsedUrl

    init() throws {
        siteUrl = try ParsedUrl.parse(input: "https://example.com")
        directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("LocalContentTests-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    }

    deinit {
        try? FileManager.default.removeItem(at: directory)
    }

    private func openService() throws -> LocalContentService {
        let store = try WpLocalContentStore(path: databasePath)
        _ = try store.performMigrations()
        return LocalContentService(store: store)
    }

    private func common(
        title: String,
        status: StatusSetting = .draft(password: nil),
        publishDate: PublishDate = .immediately,
        author: UserId? = nil,
        featuredImage: FeaturedImage = .none,
        commentStatus: PostCommentStatus? = nil
    ) -> WorkingCopyCommonFields {
        WorkingCopyCommonFields(
            title: title,
            content: "<!-- wp:paragraph --><p>\(title)</p><!-- /wp:paragraph -->",
            excerpt: "",
            slug: nil,
            status: status,
            publishDate: publishDate,
            author: author,
            featuredImage: featuredImage,
            commentStatus: commentStatus,
            pingStatus: nil
        )
    }

    private func newPost(on site: SiteIdentity) -> WorkingCopyRemoteTarget {
        WorkingCopyRemoteTarget(site: site, remotePost: nil)
    }

    private func post(title: String) -> PostWorkingCopyFields {
        PostWorkingCopyFields(common: common(title: title), categories: [], tags: [], format: .standard, sticky: false)
    }

    @Test func openingAnInMemoryDatabaseThrows() {
        #expect(throws: LocalContentError.self) {
            try WpLocalContentStore(path: ":memory:")
        }
    }

    @Test func savedPostWorkingCopySurvivesReopeningTheDatabase() throws {
        let edited = PostWorkingCopyFields(
            common: common(
                title: "Edited",
                status: .custom(status: "in-review", password: "secret"),
                publishDate: .at(date: Date(timeIntervalSince1970: 1_900_000_000)),
                author: 7,
                featuredImage: .remote(mediaId: 42),
                commentStatus: .closed
            ),
            categories: [1, 5],
            tags: [.existing(id: 3), .new(name: "not on the site yet")],
            format: .aside,
            sticky: true
        )

        let target = newPost(on: SiteIdentity(url: siteUrl, wordpressComSiteId: 123_456_789))

        let service = try openService()
        let created = try service.createPostWorkingCopy(remoteTarget: target, fields: post(title: "Original"))
        let id = created.id
        try service.savePostWorkingCopy(id: id, fields: edited)
        let read = try #require(try openService().readPostWorkingCopy(id: id))

        #expect(read.fields == edited)
        #expect(read.remoteTarget == target)
    }

    @Test func savedPageWorkingCopySurvivesReopeningTheDatabase() throws {
        let target = newPost(on: SiteIdentity(url: siteUrl, wordpressComSiteId: nil))

        let service = try openService()
        let created = try service.createPageWorkingCopy(
            remoteTarget: target,
            fields: PageWorkingCopyFields(common: common(title: "Original page"), parent: nil)
        )
        var edited = created.fields
        edited.common.title = "Edited page"
        edited.parent = 3
        try service.savePageWorkingCopy(id: created.id, fields: edited)
        let read = try #require(try openService().readPageWorkingCopy(id: created.id))

        #expect(read.fields == edited)
        #expect(read.remoteTarget == target)
    }

    @Test func savingAPageAsAPostThrows() throws {
        let service = try openService()
        let page = try service.createPageWorkingCopy(
            remoteTarget: newPost(on: SiteIdentity(url: siteUrl, wordpressComSiteId: nil)),
            fields: PageWorkingCopyFields(common: common(title: "A page"), parent: nil)
        )
        let post = post(title: "Title")

        #expect(throws: LocalContentError.self) {
            try service.savePostWorkingCopy(id: page.id, fields: post)
        }
    }
}
