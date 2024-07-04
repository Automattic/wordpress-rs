import SwiftUI

@main
struct ExampleApp: App {

    @StateObject
    var loginManager = LoginManager()

    var body: some Scene {
        WindowGroup {
            if loginManager.isLoggedIn {
                NavigationView {
                    // The first column is the sidebar.
                    RootListView()

                    // Initial content of the second column.
                    EmptyView()

                    // Initial content for the third column.
                    Text("Select a category of settings in the sidebar.")
                }.toolbar(content: {
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
            } else {
                LoginView()
            }
        }
        .environmentObject(loginManager)
    }
}
