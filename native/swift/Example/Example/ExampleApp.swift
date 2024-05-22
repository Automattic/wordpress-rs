import SwiftUI

@main
struct ExampleApp: App {

    @StateObject
    var loginManager = LoginManager()

    var body: some Scene {
        WindowGroup {
            if loginManager.isLoggedIn {
                ContentView(viewModel: UserListViewModel(loginManager: self.loginManager))
            } else {
                LoginView()
            }
        }
        .environmentObject(loginManager)
    }
}
