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
                let loginClient = WordPressLoginClient(urlSession: .shared)
                let discoveryResult = try await loginClient.discoverLoginUrl(for: url)

                guard
                    let authURLString = discoveryResult.apiDetails.findApplicationPasswordsAuthenticationUrl(),
                    let authURL = URL(string: authURLString)
                else {
                    abort() // TODO: Better error handling
                }
                debugPrint(authURL)
                let loginDetails = try await displayLoginView(withAuthenticationUrl: authURL)
                debugPrint(loginDetails)
                try await loginManager.setLoginCredentials(to: loginDetails)
            } catch let err {
                handleLoginError(err)
            }
        }
    }

    private func displayLoginView(withAuthenticationUrl authURL: URL) async throws -> WpApiApplicationPasswordDetails {
        var appNameValue = "WordPress SDK Example App"

        #if os(macOS)
        if let deviceName = Host.current().localizedName {
            appNameValue += " - (\(deviceName))"
        }
        #else
        let deviceName = UIDevice.current.name
        appNameValue += " - (\(deviceName))"
        #endif

        var mutableAuthURL = authURL

        mutableAuthURL.append(queryItems: [
            URLQueryItem(name: "app_name", value: appNameValue),
            URLQueryItem(name: "app_id", value: "00000000-0000-4000-8000-000000000000"),
            URLQueryItem(name: "success_url", value: "exampleauth://login")
        ])

        let urlWithToken = try await webAuthenticationSession.authenticate(
            using: mutableAuthURL,
            callbackURLScheme: "exampleauth"
        )

        guard let loginDetails = try WordPressAPI.Helpers.extractLoginDetails(from: urlWithToken) else {
            debugPrint("Unable to parse login details")
            abort()
        }

        return loginDetails
    }

    private func handleLoginError(_ error: Error) {
        self.isLoggingIn = false
        self.loginError = error.localizedDescription
    }
}

class AuthenticationHelper: NSObject, ASWebAuthenticationPresentationContextProviding {
    // swiftlint:disable force_cast
    func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
        #if os(macOS)
        ASPresentationAnchor(contentViewController: NSApp.windows.first!.contentViewController!)
        #elseif os(iOS)
        ASPresentationAnchor(windowScene: UIApplication.shared.connectedScenes.first as! UIWindowScene)
        #endif
    }
    // swiftlint:enable force_cast
}

// Stuff that should be Rust code

// func findApiEndpoint(in bytes: Data) -> URL {
//
// }
