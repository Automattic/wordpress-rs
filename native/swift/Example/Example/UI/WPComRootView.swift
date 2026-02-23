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
            NavigationSplitView {
                NavigationStack {
                    if isLoadingInitialData {
                        ProgressView()
                    } else {
                        RootListView(items: rootListItems.grouped)
                    }
                }
            } detail: {
                Text("Logged in")
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
        } else {
            ContentUnavailableView {
                Text("Not logged in")
            } actions: {
                Button(action: self.loginToWPCom, label: {
                    Text("Sign in to WordPress.com")
                        .padding(.horizontal)
                }).buttonStyle(.borderedProminent)
            }
        }
    }

    private func loginToWPCom() {
        Task {
            do {
                let redirectUri = URL(string: "x-wordpress-app://oauth2-callback")!

                let url = WPComApiClient.OAuth2.buildTokenRequestUrl(
                    clientId: 11,
                    redirectUri: redirectUri,
                    scope: ["global"]
                )

                let callbackUrl = try await webAuthenticationSession.authenticate(
                    using: url,
                    callbackURLScheme: "x-wordpress-app"
                )

                let tokenResponse = try WPComApiClient.OAuth2.parseTokenResponse(url: callbackUrl)

                let client = WPComApiClient(
                    authentication: .none,
                    middlewarePipeline: MiddlewarePipeline(middlewares: [DebugMiddleware()])
                )

                let requestParams = TokenRequestParameters(
                    clientId: UInt64(ProcessInfo.processInfo.environment["WPCOM_CLIENT_ID"]!)!,
                    clientSecret: ProcessInfo.processInfo.environment["WPCOM_CLIENT_SECRET"]!,
                    code: tokenResponse.code,
                    redirectUri: redirectUri.absoluteString
                )

                let response = try await client.oauth2.requestToken(params: requestParams)
                try self.loginManager.setWpComLoginCredentials(to: response.data.accessToken)

            } catch {
                self.error = error
            }
        }
    }

    private func logOutOfWPCom() {
        do {
            try self.loginManager.logoutWpCom()
        } catch {
            self.error = error
        }
    }
}

#Preview {
    WPComRootView()
        .environmentObject(WPComService(loginManager: try! LoginManager()))
}
