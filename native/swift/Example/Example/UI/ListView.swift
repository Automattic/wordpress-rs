import SwiftUI

struct ListView: View {

    @State
    var viewModel: ListViewModel

    var body: some View {
        List(viewModel.listItems.values.sorted(), id: \.id) { item in
            VStack(alignment: .leading) {
                Text(item.title).font(.headline)
                Text(item.subtitle).font(.footnote)
            }
        }
        .alert(
            isPresented: $viewModel.shouldPresentAlert,
            error: viewModel.error,
            actions: { error in // 2
                if let suggestion = error.recoverySuggestion {
                    Button(
                        suggestion,
                        action: {
                            // Recover from an error
                        }
                    )
                }
            },
            message: { error in // 3
                if let failureReason = error.failureReason {
                    Text(failureReason)
                } else {
                    Text("Something went wrong")
                }
            }
        )
        .task {
            await viewModel.task()
        }
    }
}

#Preview {

    let viewModel = TaskListViewModel(dataCallback: {
        [
            ListViewData(id: "1234", title: "Item 1", subtitle: "Subtitle", fields: [:])
        ]
    })

    return ListView(viewModel: viewModel)
}
