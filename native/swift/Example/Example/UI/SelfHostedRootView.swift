import SwiftUI

struct SelfHostedRootView: View {

    @EnvironmentObject
    var loginManager: LoginManager

    private let selfHostedService = SelfHostedService()

    @Environment(\.webAuthenticationSession)
    private var webAuthenticationSession

    @State
    private var error: Error?

    @State
    var rootListItems: [RootListData] = []

    @State
    var isLoadingInitialData: Bool = true

    @State
    var showUploadView = false

    var body: some View {
        if loginManager.isLoggedIn {
            NavigationView {
                if isLoadingInitialData {
                    ProgressView()
                } else {
                    // The first column is the sidebar.
                    RootListView(items: rootListItems.grouped)
                }

                // Initial content of the second column.
                EmptyView()

                // Initial content for the third column.
                Text("Select a category of settings in the sidebar.")
            }
            .sheet(isPresented: $showUploadView) {
                UploadView()
            }
            .toolbar(content: {
                ToolbarItem(placement: toolbarItemPlacement) {
                    Button("Log Out") {
                        Task {
                            await loginManager.logout()
                        }
                    }
                }
                ToolbarItem(placement: toolbarItemPlacement) {
                    Button("Add Media File") {
                        showUploadView = true
                    }
                }
            })
            .task {
                do {
                    self.rootListItems = try await selfHostedService.loadRootListItems()
                    self.isLoadingInitialData = false
                } catch {
                    debugPrint(error.localizedDescription)
                }
            }

        } else {
            LoginView()
        }
    }

    var toolbarItemPlacement: ToolbarItemPlacement {
        #if os(macOS)
        .automatic
        #else
        .bottomBar
        #endif
    }
}

#Preview {
    SelfHostedRootView()
}
