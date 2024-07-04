import SwiftUI
import WordPressAPI

struct RootListView: View {

    let items = [
        RootListData(name: "Application Passwords", callback: Task(operation: {
            try await WordPressAPI.globalInstance.applicationPasswords.listWithEditContext(userId: 1)
                .map { $0.asListViewData }
        })),
        RootListData(name: "Users", callback: Task(operation: {
            try await WordPressAPI.globalInstance.users.listWithEditContext(params: .init())
                .map { $0.asListViewData }
        })),
        RootListData(name: "Plugins", callback: Task(operation: {
            try await WordPressAPI.globalInstance.plugins.listWithEditContext(params: .init())
                .map { $0.asListViewData }
        }))
    ]

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
                        fetchDataTask: self.item.callback
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
    let callback: Task<[ListViewData], Error>

    var id: String {
        self.name
    }
}

#Preview {
    RootListView()
}
