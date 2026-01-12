import Foundation
import WordPressAPI
import WordPressApiCache

struct SelfHostedService: Sendable {

    private let userListParams = UserListParams(perPage: 5)
    private let postListParams = PostListParams(perPage: 5)
    private let mediaListParams = MediaListParams(perPage: 5)
    private let termListParams = TermListParams(perPage: 5)

    func loadRootListItems() async throws -> [RootListData] {
        let api = try await WordPressAPI.globalInstance
        var baseData = [
            RootListData(name: "Application Passwords", callback: {
                try await api.applicationPasswords.listWithEditContext(userId: 1)
                    .data
                    .map { $0.asListViewData }
            }, category: .system),
            RootListData(name: "Users", sequence: {
                let sequence = api.users.sequenceWithEditContext(params: self.userListParams)
                return ListViewSequence(underlyingSequence: sequence)
            }, category: .system),
            RootListData(name: "Plugins", callback: {
                try await api.plugins.listWithEditContext(params: .init())
                    .data
                    .map { $0.asListViewData }
            }, category: .system),
            RootListData(name: "Post Types", callback: {
                try await api.postTypes.listWithEditContext().data.postTypes.map { _, value in
                    value.asListViewData
                }
            }, category: .system),
            RootListData(name: "Media", sequence: {
                let sequence = api.media.sequenceWithEditContext(params: self.mediaListParams)
                return ListViewSequence(underlyingSequence: sequence)
            }, category: .posts),
            RootListData(name: "Site Health Tests", callback: {
                let items: [any ListViewDataConvertable] = [
                    try await WordPressAPI.globalInstance.siteHealthTests.authorizationHeader().data,
                    try await WordPressAPI.globalInstance.siteHealthTests.backgroundUpdates().data,
                    try await WordPressAPI.globalInstance.siteHealthTests.directorySizes().data,
                    try await WordPressAPI.globalInstance.siteHealthTests.dotorgCommunication().data,
                    try await WordPressAPI.globalInstance.siteHealthTests.httpsStatus().data,
                    try await WordPressAPI.globalInstance.siteHealthTests.loopbackRequests().data,
                    try await WordPressAPI.globalInstance.siteHealthTests.pageCache().data
                ]

                return items.map { $0.asListViewData }
            }, category: .system),
            RootListData(name: "Taxonomies", callback: {
                try await api.taxonomies
                    .listWithEditContext(params: TaxonomyListParams())
                    .data
                    .taxonomyTypes
                    .map { (_, value) in
                        value.asListViewData
                    }
            }, category: .system),
            RootListData(name: "Site Settings", callback: {
                return try await api.siteSettings.retrieveWithEditContext().data.asListViewDataItems
            }, category: .system)
        ]

        let postTypes = try await api.postTypes.listWithEditContext().data
            .postTypes
            .map(\.value)
            .filter { $0.visibility.showInNavMenus }
            .filter { $0.supports.map.keys.contains(allOf: [.title, .author, .customFields]) }

        for type in postTypes {
            let collection = try await WordPressAPI.globalInstance
                .asSelfHostedService()
                .posts()
                .createPostCollectionWithEditContext(filter: AnyPostFilter())

            let sequence = DatabaseChangeNotifier.shared.startObserving(collection).map { _ in
                try await collection.loadData().map { $0.data.asListViewData }
            }

            baseData.append(RootListData(name: type.name, sequence: {
                ListViewSequence(underlyingSequence: sequence)
            }, category: .posts))
        }

        let taxonomyTypes = try await api.taxonomies.listWithEditContext(params: TaxonomyListParams()).data
            .taxonomyTypes
            .map(\.value)
            .filter(\.visibility.showInNavMenus)

        for type in taxonomyTypes {
            baseData.append(RootListData(name: type.name, sequence: {
                let sequence = try await WordPressAPI.globalInstance.terms.sequenceWithEditContext(
                    type: .custom(type.restBase),
                    params: self.termListParams
                )

                return ListViewSequence(underlyingSequence: sequence)
            }, category: .taxonomies))
        }

        baseData.append(RootListData(name: "Post Statuses", callback: {
            try await WordPressAPI.globalInstance.postStatuses.listWithEditContext()
                .data
                .postStatuses
                .map(\.value.asListViewData)
        }, category: .posts))

        baseData.append(RootListData(name: "Navigations", callback: {
            try await WordPressAPI.globalInstance.navigations.listWithEditContext(params: NavigationListParams())
                .data
                .map(\.asListViewData)
        }, category: .navigation))

        baseData.append(RootListData(name: "Menus", callback: {
            try await WordPressAPI.globalInstance.navMenus.listWithEditContext(params: NavMenuListParams())
                .data
                .map(\.asListViewData)
        }, category: .navigation))

        baseData.append(RootListData(name: "Menu Items", sequence: {
            let sequence = try await WordPressAPI.globalInstance.navMenuItems
                .sequenceWithEditContext(params: NavMenuItemListParams())

            return ListViewSequence(underlyingSequence: sequence)
        }, category: .navigation))

        baseData.append(RootListData(name: "Menu Locations", callback: {
            try await WordPressAPI.globalInstance.menuLocations.listWithEditContext()
                .data
                .locations
                .map(\.value.asListViewData)
        }, category: .navigation))

        return baseData
    }

}
