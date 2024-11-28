import Foundation
import WordPressAPI
import WordPressAPIInternal

struct ListViewData: Identifiable, Comparable, Hashable {
    let id: String
    let title: String
    let subtitle: String
    let fields: [String: String]

    static func < (lhs: ListViewData, rhs: ListViewData) -> Bool {
        lhs.title < rhs.title
    }
}

protocol ListViewDataConvertable: Identifiable {
    var asListViewData: ListViewData { get }
}

extension UserWithEditContext: @retroactive Identifiable {}
extension UserWithEditContext: ListViewDataConvertable {
    var asListViewData: ListViewData {
        ListViewData(id: "user-\(self.id)", title: self.name, subtitle: self.email, fields: [
            "First Name": self.firstName,
            "Last Name": self.lastName,
            "Email": self.email
        ])
    }
}

extension UserWithViewContext: @retroactive Identifiable {}
extension UserWithViewContext: ListViewDataConvertable {
    var asListViewData: ListViewData {
        ListViewData(id: "user-\(self.id)", title: self.name, subtitle: self.slug, fields: [
            "Name": self.name
        ])
    }
}

extension UserWithEmbedContext: @retroactive Identifiable {}
extension UserWithEmbedContext: ListViewDataConvertable {
    var asListViewData: ListViewData {
        ListViewData(id: "user-\(self.id)", title: self.name, subtitle: self.slug, fields: [
            "Name": self.name
        ])
    }
}

extension PluginWithEditContext: @retroactive Identifiable {}
extension PluginWithEditContext: ListViewDataConvertable {
    public var id: String {
        self.plugin.slug
    }

    var asListViewData: ListViewData {
        ListViewData(id: self.plugin.slug, title: self.name, subtitle: self.version, fields: [
            "Author": self.author,
            "Author URI": self.authorUri
        ])
    }
}

extension ApplicationPasswordWithEditContext: @retroactive Identifiable {}
extension ApplicationPasswordWithEditContext: ListViewDataConvertable {
    public var id: String {
        self.uuid.uuid
    }

    var creationDateString: String {
        guard let date = Date.fromWordPressDate(self.created) else {
            return self.created
        }

        return RelativeDateTimeFormatter().string(for: date) ?? self.created
    }

    var asListViewData: ListViewData {
        ListViewData(id: self.uuid.uuid, title: self.name, subtitle: creationDateString, fields: [
            "Created": creationDateString
        ])
    }
}

extension WpSiteHealthTest: @retroactive Identifiable {}
extension SiteHealthTest: ListViewDataConvertable {
    public var id: String {
        self.label
    }

    var asListViewData: ListViewData {
        ListViewData(id: self.label, title: self.label, subtitle: self.status, fields: [:])
    }
}

extension WpSiteHealthDirectorySizes: @retroactive Identifiable {}
extension SiteHealthDirectorySizes: ListViewDataConvertable {
    public var id: String {
        [
            self.databaseSize.size,
            self.fontsSize.size,
            self.pluginsSize.size,
            self.themesSize.size,
            self.totalSize.size,
            self.uploadsSize.size,
            self.wordpressSize.size
        ].joined(separator: "-")
    }

    var asListViewData: ListViewData {
        ListViewData(
            id: self.id,
            title: "Site Health Directory Sizes",
            subtitle: "Total Size: \(totalSize.size)",
            fields: [
                "Database Size": databaseSize.size,
                "Fonts Size": fontsSize.size,
                "Plugins Size": pluginsSize.size,
                "Themes Size": themesSize.size,
                "Total Size": totalSize.size,
                "Uploads Size": uploadsSize.size,
                "WordPress Size": wordpressSize.size
            ]
        )
    }
}

extension PostTypeDetailsWithViewContext: @retroactive Identifiable {}
extension PostTypeDetailsWithViewContext: ListViewDataConvertable {
    public var id: String {
        self.slug
    }

    var asListViewData: ListViewData {
        ListViewData(id: self.id, title: self.name, subtitle: self.slug, fields: [:])
    }
}

extension SiteSettingsWithEditContext {

    var asListViewDataItems: [ListViewData] {
        [
            "Date Format": self.dateFormat,
            "Default Post Format": self.defaultPostFormat,
            "Description": self.description,
            "Email": self.email,
            "Language": self.language,
            "Show on Front": self.showOnFront,
            "Time Format": self.timeFormat,
            "Timezone": self.timezone,
            "Title": self.title,
            "URL": self.url
        ].map { key, value in
            ListViewData(id: key, title: key, subtitle: value, fields: [:])
        }
    }
}

extension PostWithEditContext: @retroactive Identifiable {}
extension PostWithEditContext: ListViewDataConvertable {
    public var id: String {
        self.slug
    }

    var asListViewData: ListViewData {
        ListViewData(id: self.id, title: self.title.raw, subtitle: self.slug, fields: [:])
    }
}

extension MediaWithEditContext: @retroactive Identifiable, ListViewDataConvertable {
    public var id: String {
        self.slug
    }

    var asListViewData: ListViewData {
        ListViewData(id: self.id, title: self.title.raw, subtitle: String(describing: self.mediaDetails), fields: [:])
    }
}

extension [PostWithEditContext] {
    func asListViewData() -> [ListViewData] {
        self.map { $0.asListViewData }
    }
}

extension [MediaWithEditContext] {
    func asListViewData() -> [ListViewData] {
        self.map { $0.asListViewData }
    }
}

extension [ListViewDataConvertable] {
    func asListViewData() -> [ListViewData] {
        self.map { $0.asListViewData }
    }
}
