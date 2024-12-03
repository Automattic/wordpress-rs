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

    private let authenticationHelper = AuthenticationHelper()

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
                let loginClient = WordPressLoginClient(urlSession: .shared)
                let loginDetails = try await loginClient.login(
                    site: url,
                    appName: "WordPress SDK Example App",
                    appId: nil
                )
                debugPrint(loginDetails)
                try await loginManager.setLoginCredentials(to: loginDetails)
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

class AuthenticationHelper: NSObject, ASWebAuthenticationPresentationContextProviding {
    func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
        ASPresentationAnchor()
    }
}
