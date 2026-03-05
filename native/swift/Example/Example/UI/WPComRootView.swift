import SwiftUI
import AuthenticationServices
import WordPressAPI

struct WPComRootView: View {

    @EnvironmentObject
    var loginManager: LoginManager

    @Environment(\.webAuthenticationSession)
    private var webAuthenticationSession

    @EnvironmentObject
    private var wpcomService: WPComService

    @State
    var isLoadingInitialData: Bool = true

    @State
    var rootListItems: [RootListData] = []

    @State
    private var error: Error?

    var body: some View {
        if loginManager.isLoggedInToWpCom {
            NavigationView {
                if isLoadingInitialData {
                    ProgressView()
                } else {
                    RootListView(items: rootListItems.grouped)
                }

                EmptyView()

                Text("Select an item from the sidebar.")
            }
            .toolbar {
                Button(action: self.logOutOfWPCom) {
                    Text("Sign Out")
                }
            }.task {
                do {
                    self.rootListItems = try await wpcomService.loadRootListItems()
                    self.isLoadingInitialData = false
                } catch {
                    debugPrint(error.localizedDescription)
                }
            }
        } else if loginManager.wpComOAuthConfiguration != nil {
            ContentUnavailableView {
                Text("Not logged in")
            } actions: {
                Button(action: self.loginToWPCom, label: {
                    Text("Sign in to WordPress.com")
                        .padding(.horizontal)
                }).buttonStyle(.borderedProminent)
            }
        } else {
            ContentUnavailableView {
                Text("WordPress.com credentials not configured")
            } description: {
                Text("Add a wp_com_test_credentials.json file to the repository root and rebuild.")
            }
        }
    }

    private func loginToWPCom() {
        guard let configuration = loginManager.wpComOAuthConfiguration else { return }

        Task {
            do {
                try await loginManager.logInToWpCom(
                        configuration: configuration,
                        webAuthenticationSession: webAuthenticationSession
                )
            } catch {
                self.error = error
            }
        }
    }

    private func logOutOfWPCom() {
        Task {
            do {
                try await self.loginManager.logoutWpCom()
            } catch {
                self.error = error
            }
        }
    }
}

#Preview {
    // swiftlint:disable force_try
    WPComRootView()
        .environmentObject(WPComService(loginManager: try! LoginManager()))
    // swiftlint:enable force_try
}
