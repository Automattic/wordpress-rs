import SwiftUI

struct SelfHostedRootView: View {

    @EnvironmentObject
    var loginManager: LoginManager

    @EnvironmentObject
    var selfHostedService: SelfHostedService

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
                UploadView(viewModel: UploadViewModel(loginManager: loginManager))
            }
            .toolbar(content: {
                ToolbarItem(placement: toolbarItemPlacement) {
                    Button("Log Out") {
                        do {
                            try loginManager.logout()
                        } catch {
                            debugPrint(error.localizedDescription)
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
        .environmentObject(SelfHostedService(loginManager: try! LoginManager()))
}
