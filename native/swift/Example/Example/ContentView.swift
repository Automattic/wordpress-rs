import SwiftUI
import wordpress_api

struct ContentView: View {

    @State
    private var viewModel = PostListViewModel.shared

    var body: some View {
        Group {
            if viewModel.posts.isEmpty {
                VStack {
                    ProgressView().progressViewStyle(.circular)
                    Text("Fetching Posts")
                }
                .padding()
            } else {
                List(viewModel.posts) { post in
                    Text(post.title?.raw ?? "")
                }
            }
        }
        .onAppear(perform: viewModel.startFetchingPosts)
//        .onDisappear(perform: viewModel.stopFetchingPost)
        .alert(
            isPresented: $viewModel.shouldPresentAlert,
            error: viewModel.error,
            actions: { error in // 2
                if let suggestion = error.recoverySuggestion {
                    Button(suggestion, action: {
                        // Recover from an error
                    })
                }
            }, message: { error in // 3
            if let failureReason = error.failureReason {
                Text(failureReason)
            } else {
                Text("Something went wrong")
            }
        })
    }
}

#Preview {
    ContentView()
}