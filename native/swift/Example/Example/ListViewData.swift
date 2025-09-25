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

protocol ListViewDataConvertable {
    var asListViewData: ListViewData { get }
}

extension UserWithEditContext: ListViewDataConvertable {
    var asListViewData: ListViewData {
        ListViewData(id: "user-\(self.id)", title: self.name, subtitle: self.email, fields: [
            "First Name": self.firstName,
            "Last Name": self.lastName,
            "Email": self.email
        ])
    }
}

extension UserWithViewContext: ListViewDataConvertable {
    var asListViewData: ListViewData {
        ListViewData(id: "user-\(self.id)", title: self.name, subtitle: self.slug, fields: [
            "Name": self.name
        ])
    }
}

extension UserWithEmbedContext: ListViewDataConvertable {
    var asListViewData: ListViewData {
        ListViewData(id: "user-\(self.id)", title: self.name, subtitle: self.slug, fields: [
            "Name": self.name
        ])
    }
}

extension PluginWithEditContext: ListViewDataConvertable {
    var asListViewData: ListViewData {
        ListViewData(id: self.plugin.slug, title: self.name, subtitle: self.version, fields: [
            "Author": self.author,
            "Author URI": self.authorUri
        ])
    }
}

extension ApplicationPasswordWithEditContext: ListViewDataConvertable {
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

extension SiteHealthTest: ListViewDataConvertable {
    var asListViewData: ListViewData {
        ListViewData(id: self.label, title: self.label, subtitle: self.status, fields: [:])
    }
}

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

extension PostTypeDetailsWithEditContext: ListViewDataConvertable {
    var asListViewData: ListViewData {
        ListViewData(id: self.slug, title: self.name, subtitle: self.slug, fields: [
            "REST Base": self.restBase,
            "Show in Nav": self.visibility.showInNavMenus.description
        ])
    }
}

extension TaxonomyTypeDetailsWithEditContext: ListViewDataConvertable {
    var asListViewData: ListViewData {
        ListViewData(id: self.slug, title: self.name, subtitle: self.restBase, fields: [
            "REST Base": self.restBase,
            "Show in Nav": self.visibility.showInNavMenus.description

        ])
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

extension AnyPostWithEditContext: ListViewDataConvertable {
    var asListViewData: ListViewData {
        ListViewData(id: self.slug, title: self.title.rendered, subtitle: self.slug, fields: [:])
    }
}

extension MediaWithEditContext: ListViewDataConvertable {
    var asListViewData: ListViewData {
        let details = self.mediaDetails.parseAsMimeType(mimeType: self.mimeType)
        return ListViewData(
            id: self.slug,
            title: details.emoji + " " + (URL(string: self.sourceUrl)?.lastPathComponent ?? "<invalid-source-url>"),
            subtitle: self.title.rendered,
            fields: details.fields
        )
    }
}

extension [AnyPostWithEditContext] {
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

private extension Optional<MediaDetailsPayload> {
    var emoji: String {
        switch self {
        case .audio:
            "🔊"
        case .image:
            "🌆"
        case .video:
            "🎥"
        case .document:
            "📁"
        case nil:
            "❓"
        }
    }

    var fields: [String: String] {
        var fields = [String: String]()

        switch self {
        case let .audio(audio):
            fields["Duration"] = "\(audio.length) seconds"
            fields["File size"] = "\(audio.fileSize) bytes"
        case let .image(image):
            fields["Size"] = "\(image.width)x\(image.height) pixels"
            fields["File size"] = "\(image.fileSize) bytes"
        case let .video(video):
            fields["Size"] = "\(video.width)x\(video.height) pixels"
            fields["Duration"] = "\(video.length) seconds"
            fields["File size"] = "\(video.fileSize) bytes"
        case let .document(doc):
            fields["File size"] = "\(doc.fileSize) bytes"
        case nil:
            break
        }

        return fields
    }
}
