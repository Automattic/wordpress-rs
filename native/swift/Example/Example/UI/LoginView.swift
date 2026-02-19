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

            if loginManager.wpComOAuthConfiguration != nil {
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
                let siteDetails = try await loginClient.details(ofSite: url)

                if let applicationPasswordUrl = siteDetails.authentication.loginURL(for: application) {
                    let callbackUrl = try await self.webAuthenticationSession.authenticate(
                        using: applicationPasswordUrl,
                        callbackURLScheme: "x-wordpress-app"
                    )

                    let loginDetails = try loginClient.credentials(from: callbackUrl)
                    try await loginManager
                        .setLoginCredentials(to: loginDetails, apiRootURL: siteDetails.parsedSiteUrl.asURL())
                }

                if let endpoints = siteDetails.authentication.oauthEndpoints {
                    if let configuration = loginManager.oauthRegistry.findConfiguration(endpoints: endpoints) {
                        guard let host = siteDetails.parsedSiteUrl.asURL().host() else {
                            preconditionFailure("Invalid site details response")
                        }

                        try await loginManager.logInToWpCom(
                            configuration: configuration,
                            webAuthenticationSession: webAuthenticationSession,
                            blogId: .slug(value: host)
                        )
                    }
                }
            } catch let err {
                handleLoginError(err)
            }
        }
    }

    func startLoginWithWPCom() {
        guard let configuration = loginManager.wpComOAuthConfiguration else { return }

        self.loginError = nil
        self.isLoggingIn = true

        self.loginTask = Task {
            do {
                try await loginManager.logInToWpCom(
                    configuration: configuration,
                    webAuthenticationSession: webAuthenticationSession
                )
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
