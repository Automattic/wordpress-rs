import Foundation
import WordPressAPI

struct ListViewData: Identifiable {
    let id: String
    let title: String
    let subtitle: String
    let fields: [String: String]
}

protocol ListViewDataConvertable: Identifiable {
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

extension SiteHealthTest: ListViewDataConvertable {
    var asListViewData: ListViewData {
        ListViewData(id: self.label, title: self.label, subtitle: self.status, fields: [:])
    }

    public var id: String {
        self.label
    }
}

extension PostTypeDetailsWithViewContext: ListViewDataConvertable {
    public var id: String {
        self.slug
    }

    var asListViewData: ListViewData {
        ListViewData(id: self.id, title: self.name, subtitle: self.slug, fields: [:])
    }
}
