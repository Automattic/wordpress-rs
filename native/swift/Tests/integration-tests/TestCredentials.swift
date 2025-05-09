import Foundation

struct TestCredentials: Decodable {
    let siteUrl: String
    let adminUsername: String
    let adminPassword: String
    let adminPasswordUuid: String
    let subscriberUsername: String
    let subscriberPassword: String
    let subscriberPasswordUuid: String
    let authorUsername: String
    let authorPassword: String
    let passwordProtectedPostId: Int
    let passwordProtectedPostPassword: String
    let passwordProtectedPostTitle: String
    let passwordProtectedCommentId: Int
    let passwordProtectedCommentAuthor: String
    let trashedPostId: Int
    let firstPostDateGmt: String
    let wordpressCoreVersion: String

    enum CodingKeys: String, CodingKey {
        case siteUrl = "site_url"
        case adminUsername = "admin_username"
        case adminPassword = "admin_password"
        case adminPasswordUuid = "admin_password_uuid"
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
        let url = URL(string: "../../../../test_credentials.json", relativeTo: URL(fileURLWithPath: #filePath))!.absoluteURL
        return try! JSONDecoder().decode(Self.self, from: Data(contentsOf: url))
    }
}
