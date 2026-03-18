import SwiftUI
import WordPressApiCache
import WordPressAPI
import Combine

struct RootListView: View {

    let items: [RootListData.Category: [RootListData]]

    var body: some View {
        List {
            ForEach(RootListData.Category.allCases) { category in
                if let items = items[category] {
                    Section(category.name) {
                        ForEach(items) { item in
                            RootListViewItem(item: item)
                        }
                    }
                }
            }
        }
    }
}

struct RootListViewItem: View {
    let item: RootListData

    var body: some View {
        switch item {
        case .callback(let name, let fetchDataTask, _):
            VStack(alignment: .leading, spacing: 4.0) {
                NavigationLink {
                    ListView(
                        viewModel: TaskListViewModel(dataCallback: fetchDataTask)
                    )
                } label: {
                    Text(name)
                }
            }

        case .sequence(let name, let sequenceProvider, _):
            VStack(alignment: .leading, spacing: 4.0) {
                NavigationLink {
                    ListView(
                        viewModel: SequenceListViewModel(sequenceProvider: sequenceProvider)
                    )
                } label: {
                    Text(name)
                }
            }
        case .collection(let name, let cachedResults, let fetchedResults, _):
            VStack(alignment: .leading) {
                NavigationLink {
                    ListView(
                        viewModel: CollectionListViewModel(
                            cachedResults: cachedResults,
                            fetchedResults: fetchedResults
                        )
                    )
                } label: {
                    Text(name)
                }
            }
        }
    }
}

enum RootListData: Identifiable, Sendable {

    enum Category: Hashable, Identifiable, CaseIterable {
        case posts
        case taxonomies
        case navigation
        case system

        var id: String {
            name
        }

        var name: String {
            switch self {
            case .posts: "Posts"
            case .taxonomies: "Taxonomies"
            case .navigation: "Navigation"
            case .system: "System"
            }
        }
    }

    case callback(String, TaskListViewModel.FetchDataTask, Category)
    case sequence(String, SequenceListViewModel.SequenceProvider, Category)
    case collection(
        String,
        CollectionListViewModel.CachedResultProvider,
        CollectionListViewModel.FetchedResultsProvider,
        Category
    )

    var id: String {
        switch self {
        case .callback(let id, _, _): id
        case .sequence(let id, _, _): id
        case .collection(let id, _, _, _): id
        }
    }

    var category: Category {
        switch self {
        case .callback(_, _, let category): category
        case .sequence(_, _, let category): category
        case .collection(_, _, _, let category): category
        }
    }

    init(name: String, callback: @escaping TaskListViewModel.FetchDataTask, category: Category) {
        self = .callback(name, callback, category)
    }

    init(name: String, category: Category, callback: @escaping TaskListViewModel.FetchDataTask) {
        self = .callback(name, callback, category)
    }

    init(name: String, sequence: @escaping SequenceListViewModel.SequenceProvider, category: Category) {
        self = .sequence(name, sequence, category)
    }

    init(
        name: String,
        cachedDataProvider: @escaping CollectionListViewModel.CachedResultProvider,
        fetchedDataProvider: @escaping CollectionListViewModel.FetchedResultsProvider,
        category: Category
    ) {
        self = .collection(
            name,
            cachedDataProvider,
            fetchedDataProvider,
            category
        )
    }
}

extension [RootListData] {
    var grouped: [RootListData.Category: [RootListData]] {
        var groups: [RootListData.Category: [RootListData]] = [:]

        for item in self {
            if groups[item.category] == nil {
                groups[item.category] = []
            }

            groups[item.category]?.append(item)
        }

        return groups
    }
}

#Preview {
    RootListView(items: [:])
}
