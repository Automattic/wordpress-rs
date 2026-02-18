import Foundation
import SwiftUI
import WordPressAPI
import AuthenticationServices

struct LoginView: View {

    @State
    private var url: String = ""

    @State
    private var isLoggingIn: Bool = false

    @State
    private var loginError: String?

    @State
    private var loginTask: Task<Void, Error>?

    @Environment(\.webAuthenticationSession)
    private var webAuthenticationSession

    @EnvironmentObject
    var loginManager: LoginManager

    var body: some View {
        VStack {
            if let loginError {
                Text(loginError)
            }

            TextField(text: $url) {
                Text("Website URL")
            }.onSubmit {
                self.startLogin()
            }
            #if os(iOS)
            .keyboardType(.URL)
            .autocorrectionDisabled()
            .textInputAutocapitalization(.never)
            #endif
        }
        .padding()
        .toolbar {
            Button(action: self.startLogin, label: {
                if isLoggingIn {
                    ProgressView()
                        .progressViewStyle(.circular)
                        .controlSize(.small)
                        .padding()
                } else {
                    Text("Sign In")
                        .padding(.horizontal)
                }
            })

            if loginManager.wpComOAuthCredentials != nil {
                Button(action: self.startLoginWithWPCom, label: {
                    Text("Sign in with WordPress.com")
                })
            }
        }
    }

    func startLogin() {
        self.loginError = nil
        self.isLoggingIn = true

        self.loginTask = Task {
            do {
                let application = Application(
                    // swiftlint:disable:next force_try
                    id: try! WpUuid.parse(input: "DBD2ADE9-3047-4C0B-AC66-F390F3EAA525"),
                    name: "WordPress-rs Example App for iOS",
                    callbackUrl: "x-wordpress-app://login-callback"
                )

                let loginClient = WordPressLoginClient(urlSession: .shared)
                let details = try await loginClient.details(ofSite: url)

                let callbackUrl = try await self.webAuthenticationSession.authenticate(
                    using: details.loginURL(for: application),
                    callbackURLScheme: "x-wordpress-app"
                )

                let loginDetails = try loginClient.credentials(from: callbackUrl)
                try await loginManager.setLoginCredentials(to: loginDetails, apiRootURL: details.apiRootUrl.asURL())
            } catch let err {
                handleLoginError(err)
            }
        }
    }

    func startLoginWithWPCom() {
        guard let credentials = loginManager.wpComOAuthCredentials else { return }

        self.loginError = nil
        self.isLoggingIn = true

        self.loginTask = Task {
            do {
                let redirectUri = URL(string: "x-wordpress-app://oauth2-callback")!

                let url = WPComApiClient.OAuth2.buildTokenRequestUrl(
                    clientId: credentials.clientId,
                    redirectUri: redirectUri,
                    scope: ["global"]
                )

                let callbackUrl = try await self.webAuthenticationSession.authenticate(
                    using: url,
                    callbackURLScheme: "x-wordpress-app"
                )

                let tokenResponse = try WPComApiClient.OAuth2.parseTokenResponse(url: callbackUrl)

                let client = WPComApiClient(
                    authentication: .none,
                    middlewarePipeline: MiddlewarePipeline(middlewares: [DebugMiddleware()])
                )

                let requestParams = TokenRequestParameters(
                    clientId: credentials.clientId,
                    clientSecret: credentials.clientSecret,
                    code: tokenResponse.code,
                    redirectUri: redirectUri.absoluteString
                )

                let response = try await client.oauth2.requestToken(params: requestParams)
                try self.loginManager.setWpComLoginCredentials(to: response.data.accessToken)
            } catch {
                handleLoginError(error)
            }
        }
    }

    private func handleLoginError(_ error: Error) {
        self.isLoggingIn = false
        self.loginError = error.localizedDescription
    }
}

#Preview {
    LoginView()
        .environmentObject(try! LoginManager())
}
