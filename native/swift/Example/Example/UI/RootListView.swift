import SwiftUI
import WordPressAPI

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
        VStack(alignment: .leading, spacing: 4.0) {
            NavigationLink {
                ListView(
                    viewModel: ListViewModel(
                        loginManager: LoginManager(),
                        dataCallback: self.item.callback
                    )
                )
            } label: {
                Text(item.name)
            }
        }
    }
}

struct RootListData: Identifiable {

    let name: String
    let callback: ListViewModel.FetchDataTask

    var id: String {
        self.name
    }
}

#Preview {
    RootListView(items: [])
}
