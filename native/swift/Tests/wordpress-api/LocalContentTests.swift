import Foundation
import Testing
import WordPressAPI

final class LocalContentTests {
    private let directory: URL

    private var databasePath: String {
        directory.appendingPathComponent("local_content.db").path
    }

    private let siteUrl: ParsedUrl
    private let newPostTarget: WorkingCopyTarget

    init() throws {
        siteUrl = try ParsedUrl.parse(input: "https://example.com")
        newPostTarget = WorkingCopyTarget(site: .selfHosted(url: siteUrl), remotePost: nil)
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
    ) -> WorkingCopyCommon {
        WorkingCopyCommon(
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

    private func post(title: String) -> PostWorkingCopy {
        PostWorkingCopy(common: common(title: title), categories: [], tags: [], format: .standard, sticky: false)
    }

    @Test func openingAnInMemoryDatabaseThrows() {
        #expect(throws: LocalContentError.self) {
            try WpLocalContentStore(path: ":memory:")
        }
    }

    @Test func savedPostWorkingCopySurvivesReopeningTheDatabase() throws {
        let target = WorkingCopyTarget(
            site: .wordPressCom(siteId: 123_456_789),
            remotePost: RemotePostReference(postId: 99, baseModifiedGmt: Date(timeIntervalSince1970: 1_700_000_000))
        )
        let edited = PostWorkingCopy(
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

        let service = try openService()
        let id = try service.createPostWorkingCopy(workingCopy: post(title: "Original"), target: target)
        try service.savePostWorkingCopy(id: id, workingCopy: edited)
        let read = try #require(try openService().readPostWorkingCopy(id: id))

        // `WorkingCopyTarget` isn't `Equatable` in Swift, so compare its parts.
        #expect(read.workingCopy == edited)
        #expect(read.target.remotePost == target.remotePost)
        guard case .wordPressCom(let siteId) = read.target.site else {
            Issue.record("Expected a WordPress.com site, got \(read.target.site)")
            return
        }
        #expect(siteId == 123_456_789)
    }

    @Test func savedPageWorkingCopySurvivesReopeningTheDatabase() throws {
        let edited = PageWorkingCopy(common: common(title: "Edited page"), parent: 3)

        let service = try openService()
        let id = try service.createPageWorkingCopy(
            workingCopy: PageWorkingCopy(common: common(title: "Original page"), parent: nil),
            target: newPostTarget
        )
        try service.savePageWorkingCopy(id: id, workingCopy: edited)
        let read = try #require(try openService().readPageWorkingCopy(id: id))

        #expect(read.workingCopy == edited)
        #expect(read.target.remotePost == nil)
        guard case .selfHosted(let url) = read.target.site else {
            Issue.record("Expected a self-hosted site, got \(read.target.site)")
            return
        }
        #expect(url == siteUrl)
    }

    @Test func savingAPageAsAPostThrows() throws {
        let service = try openService()
        let pageId = try service.createPageWorkingCopy(
            workingCopy: PageWorkingCopy(common: common(title: "A page"), parent: nil),
            target: newPostTarget
        )
        let post = post(title: "Title")

        #expect(throws: LocalContentError.self) {
            try service.savePostWorkingCopy(id: pageId, workingCopy: post)
        }
    }
}
