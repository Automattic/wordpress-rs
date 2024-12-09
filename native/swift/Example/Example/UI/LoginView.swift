import Foundation
import SwiftUI
import WordPressAPI
import AuthenticationServices

struct LoginView: View {

    @State
    private var url: String = ""

    @State
    private var isLoading: Bool = false

    @State
    private var loginError: String?

    @State
    private var currentTask: Task<Void, Error>?

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
                if isLoading {
                    ProgressView()
                        .progressViewStyle(.circular)
                        .controlSize(.small)
                        .padding()
                } else {
                    Button(action: self.startAutodiscovery, label: {
                        Text("Next")
                    })
                }
            }
        }
        .padding()
    }

    func startAutodiscovery() {
        self.currentTask = Task {
            self.isLoading = true

            do {
                let loginClient = WordPressLoginClient(requestExecutor: URLSession.shared)
                let loginDetails = await loginClient.autodiscoveryResult(forSite: url)
                
                debugPrint(loginDetails)
            }

            self.isLoading = false
        }
    }

    func startLogin() {
        self.loginError = nil
        self.isLoading = true

//        self.currentTask = Task {
//            do {
////                let loginClient = WordPressLoginClient(requestExecutor: URLSession.shared)
////                let loginDetails = try await loginClient.login(
////                    site: url,
////                    appName: "WordPress SDK Example App",
////                    appId: nil,
////                    contextProvider: AuthenticationHelper()
////                ).get()
////                debugPrint(loginDetails)
////                try await loginManager.setLoginCredentials(to: loginDetails)
//            } catch let err {
//                handleLoginError(err)
//            }
//        }
    }

    private func handleLoginError(_ error: Error) {
        self.isLoading = false
        self.loginError = error.localizedDescription
    }
}

class AuthenticationHelper: NSObject, ASWebAuthenticationPresentationContextProviding {
    func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
        ASPresentationAnchor()
    }

//    LoginView().environmentObject(LoginManager())
}
