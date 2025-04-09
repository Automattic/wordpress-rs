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

            HStack {
                if isLoggingIn {
                    ProgressView()
                        .progressViewStyle(.circular)
                        .controlSize(.small)
                        .padding()
                } else {
                    Button(action: self.startLogin, label: {
                        Text("Sign In")
                    })
                }
            }
        }
        .padding()
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

                let loginClient = WpLoginClient(urlSession: .shared)
                let details = try await loginClient.apiDiscovery(siteUrl: url)
                let loginUrl = try await details.loginURL(for: application)

                let callbackUrl = try await self.webAuthenticationSession.authenticate(
                    using: loginUrl,
                    callbackURLScheme: "x-wordpress-app"
                )

                let loginDetails = try loginClient.credentials(from: callbackUrl)
                try await loginManager.setLoginCredentials(to: loginDetails, apiRootURL: details.apiRootUrl.asURL())
            } catch let err {
                handleLoginError(err)
            }
        }
    }

    private func handleLoginError(_ error: Error) {
        self.isLoggingIn = false
        self.loginError = error.localizedDescription
    }
}
