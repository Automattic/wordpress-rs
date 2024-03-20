import Foundation
import SwiftUI
import wordpress_api
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

                guard var authURL = try await getLoginUrl() else {
                    return
                }

                var appNameValue = "WordPress SDK Example App"

                #if os(macOS)
                if let deviceName = Host.current().localizedName {
                    appNameValue += " - (\(deviceName))"
                }
                #else
                if let deviceName = UIDevice.current.name {
                    appNameValue += " - (\(deviceName))"
                }
                #endif

                authURL.append(queryItems: [
                    URLQueryItem(name: "app_name", value: appNameValue),
                    URLQueryItem(name: "app_id", value: "00000000-0000-4000-8000-000000000000"),
                    URLQueryItem(name: "success_url", value: "exampleauth://login")
                ])

                let urlWithToken = try await webAuthenticationSession.authenticate(
                    using: authURL,
                    callbackURLScheme: "exampleauth"
                )

                guard let loginDetails = WordPressAPI.Helpers.extractLoginDetails(from: urlWithToken) else {
                    debugPrint("Unable to parse login details")
                    abort()
                }

                try await loginManager.setLoginCredentials(to: loginDetails)
            } catch let err {
                self.isLoggingIn = false
                self.loginError = err.localizedDescription
                debugPrint(err)
            }
        }
    }

    func getLoginUrl() async throws -> URL? {
        let parsedUrl = try WordPressAPI.Helpers.parseUrl(string: url)
        debugPrint("Trying login to \(parsedUrl)")

        let apiRoot = try await WordPressAPI.findRestApiEndpointRoot(
            forSiteUrl: parsedUrl,
            using: URLSession.shared
        )
        let client = WordPressAPI(
            urlSession: .shared,
            baseUrl: apiRoot,
            authenticationStategy: .init(authToken: "")
        )
        let capabilities = try await client.getRestAPICapabilities(forApiRoot: apiRoot, using: .shared)

        guard let authenticationUrl = capabilities.authentication.first?.value.endpoints.authorization else {
            debugPrint("No authentication approaches found – unable to continue")
            abort()
        }

        return URL(string: authenticationUrl)
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
