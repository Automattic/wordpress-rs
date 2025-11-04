import Foundation

struct TestCredentials: Decodable {
    var siteUrl: String
    var adminUsername: String
    var adminPassword: String
    var adminPasswordUuid: String
    var adminAccountPassword: String
    var subscriberUsername: String
    var subscriberPassword: String
    var subscriberPasswordUuid: String
    var authorUsername: String
    var authorPassword: String
    var passwordProtectedPostId: Int
    var passwordProtectedPostPassword: String
    var passwordProtectedPostTitle: String
    var passwordProtectedCommentId: Int
    var passwordProtectedCommentAuthor: String
    var trashedPostId: Int
    var firstPostDateGmt: String
    var wordpressCoreVersion: String

    enum CodingKeys: String, CodingKey {
        case siteUrl = "site_url"
        case adminUsername = "admin_username"
        case adminPassword = "admin_password"
        case adminPasswordUuid = "admin_password_uuid"
        case adminAccountPassword = "admin_account_password"
        case subscriberUsername = "subscriber_username"
        case subscriberPassword = "subscriber_password"
        case subscriberPasswordUuid = "subscriber_password_uuid"
        case authorUsername = "author_username"
        case authorPassword = "author_password"
        case passwordProtectedPostId = "password_protected_post_id"
        case passwordProtectedPostPassword = "password_protected_post_password"
        case passwordProtectedPostTitle = "password_protected_post_title"
        case passwordProtectedCommentId = "password_protected_comment_id"
        case passwordProtectedCommentAuthor = "password_protected_comment_author"
        case trashedPostId = "trashed_post_id"
        case firstPostDateGmt = "first_post_date_gmt"
        case wordpressCoreVersion = "wordpress_core_version"
    }

    static func instance() -> Self {
        let json = URL(
                string: "../../../../test_credentials.json",
                relativeTo: URL(fileURLWithPath: #filePath)
            )!
            .absoluteURL
        // swiftlint:disable:next force_try
        return try! JSONDecoder().decode(Self.self, from: Data(contentsOf: json))
    }
}
