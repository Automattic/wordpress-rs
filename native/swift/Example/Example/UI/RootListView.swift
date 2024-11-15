import SwiftUI
import WordPressAPI
import Combine

struct RootListView: View {

    let items: [RootListData]

    var body: some View {
        List(self.items) { data in
            RootListViewItem(item: data)
        }
    }
}

struct RootListViewItem: View {
    let item: RootListData

    var body: some View {
        switch item {
        case .callback(let name, let fetchDataTask):
            VStack(alignment: .leading, spacing: 4.0) {
                NavigationLink {
                    ListView(
                        viewModel: TaskListViewModel(dataCallback: fetchDataTask)
                    )
                } label: {
                    Text(name)
                }
            }

        case .sequence(let name, let sequenceProvider):
            VStack(alignment: .leading, spacing: 4.0) {
                NavigationLink {
                    ListView(
                        viewModel: SequenceListViewModel(sequenceProvider: sequenceProvider)
                    )
                } label: {
                    Text(name)
                }
            }

            case .publisher(let name, let streamProvider):
                VStack(alignment: .leading, spacing: 4.0) {
                    NavigationLink {
                        ListView(
                            viewModel: CombineListViewModel(streamProvider: streamProvider)
                        )
                    } label: {
                        Text(name)
                    }
                }
        }
    }
}

enum RootListData: Identifiable {

    case callback(String, TaskListViewModel.FetchDataTask)
    case sequence(String, SequenceListViewModel.SequenceProvider)
    case publisher(String, CombineListViewModel.StreamProvider)

    var id: String {
        switch self {
        case .callback(let id, _): id
        case .sequence(let id, _): id
        case .publisher(let id, _): id
        }
    }

    init(name: String, callback: @escaping TaskListViewModel.FetchDataTask) {
        self = .callback(name, callback)
    }

    init(name: String, sequence: @escaping SequenceListViewModel.SequenceProvider) {
        self = .sequence(name, sequence)
    }

    init(name: String, publisher: @escaping CombineListViewModel.StreamProvider) {
        self = .publisher(name, publisher)
    }
}

#Preview {
    RootListView(items: [])
}
