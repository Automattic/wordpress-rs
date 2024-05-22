import SwiftUI
import wordpress_api

struct ContentView: View {

    @State
    private var viewModel: UserListViewModel

    @EnvironmentObject
    var loginManager: LoginManager

    init(viewModel: UserListViewModel) {
        self.viewModel = viewModel
    }

    var body: some View {
        Group {
            if viewModel.users.isEmpty {
                VStack {
                    ProgressView().progressViewStyle(.circular)
                    Text("Fetching users")
                }
                .padding()
            } else {
                List(viewModel.users) {
                    Text($0.name)
                }
            }
        }
        .onAppear(perform: viewModel.startFetching)
//        .onDisappear(perform: viewModel.stopFetching)
        .alert(
            isPresented: $viewModel.shouldPresentAlert,
            error: viewModel.error,
            actions: { error in // 2
                if let suggestion = error.recoverySuggestion {
                    Button(suggestion, action: {
                        // Recover from an error
                    })
                }
            }, message: { error in // 3
            if let failureReason = error.failureReason {
                Text(failureReason)
            } else {
                Text("Something went wrong")
            }
        }).toolbar(content: {
            #if os(macOS)
            ToolbarItem {
                Button("Log Out") {
                    Task {
                        await loginManager.logout()
                    }
                }
            }
            #else
            ToolbarItem(placement: .bottomBar) {
                Button("Log Out") {
                    Task {
                        await loginManager.logout()
                    }
                }
            }
            #endif
        })
    }
}
//
// #Preview {
//    ContentView()
// }
