Relates to https://github.com/Automattic/wordpress-rs/issues/824.

This PR adds cancellation support to the Swift wrapper. The same API (for example, `api.posts.create(...)`), which was not cancellable, can now be cancelled.

To verify this change, you can checkout the first commit (Add CancellationTests), run `make xcframework-only-macos`, then run the `CancellationTests` from Xcode. The test should fail. Checkout the last commit, repeat the same steps, the same tests should pass.

### Goal

Use this Swift code as an example:

```swift
let task = Task {
    let newPost = try await api.posts.create(...)
    viewPostDetails(newPost)
}

// a short moment later
task.cancel()
```

When you cancel the task that is creating a new post and the POST HTTP request is still going, we expect the HTTP request to be cancelled, and the following `viewPostDetails` call should not be called.

Putting this scenario into the mobile app's context, you can think of it as cancelling the publishing of a new post, replying to a comment, uploading an image, etc. When user taps a cancel button to cancel those pending jobs, we should try our best to abort the underlying HTTP request to prevent the content reaching the server. Obviously, we can't 100% guarantee that, because the request may have already reached the server when user taps the cancel button, and we can't do much from client side about that.

### Difficulties

There are several difficulties involved in supporting the native Swift task cancellation, when the task calls Rust's async functions underneath. Because both Swift's Task and Rust's Future API are involved, and they obviously work quite differently.

As I mentioned in https://github.com/Automattic/wordpress-rs/issues/824, uniffi-rs does not have built-in cancellation support. We'd need to implement the mechanism ourselves.

### Approach

First, Rust API needs to expose a `cancellation_token: CancellationToken`. This PR modified the derive macro to add the parameter to all endpoint functions.

The generated code now looks like this:

```rust
impl UsersRequestExecutor {
    // This is a new function, which supports cancellation when the `cancellation_token` parameter value is not None.
    fn list_with_view_context_cancellation(params: UserListParams, cancellation_token: Option<Arc<CancellationToken>>) {
        let request = ...
        let response = self.request_executor.execute(request, cancellation_token).await;
        return response.parse()
    }

    // This is the existing function, which does not support cancellation, since it passes `cancellation_token: None` to the function above.
    fn list_with_view_context(params: UserListParams) {
        list_with_view_context_cancellation(params, None)
    }
}
```

This change does not break the endpoint APIs. Their function signature stays the same, and they cannot be cancelled as before. The Rust code acts as a facilitator, which accepts a `CancellationToken` instance and passes it to the native implementation of `RequestExecutor`, so that the request executor can cancel HTTP requests when cancelled.

The `RequestExecutor` API is changed to support that. Both existing functions now accept a `cancellation_token: CancellationToken?` parameter. The native implementation can use `CancellationToken.register_handler` function to get a callback when `CancellationToken` gets cancelled. The implementation here is isolated within the native `RequestExecutor`, which makes the actual HTTP request cancellation relatively straightforward.

Finally, in order to provide a cancellable version of `list_with_view_context`, and keep the source code backward compatible, I have introduced a feature to prevent the `list_with_view_context` from being exported as a uniffi function. The native wrapper can then generate the `list_with_view_context` function according to the cancellable version: `list_with_view_context_cancellation`. It looks like this in Swift:

```swift
extension UsersRequestExecutor {
    func listWithViewContext(params: UserListParam) async throws {
        let token = CancellationToken()
        return try await withTaskCancellationHandler {
            try await self.listWithViewContextCancellation(params, cancellationToken: token)
        } onCancel: {
            token.cancel()
        }
    }
}
```

### Limitations

The cancellation support is opt-in. As in, the Rust functions need to accept a `CancellationToken` parameter, and make sure to pass the instance down to its call chain.

```rust
fn upload_post(cancellation_token: CancellationToken) {
    for image in images {
        media.upload_image(image, cancellation_token).await
    }

    post.create(title, content, cancellation_token).await
}
```

The macro change should cover the majority of the HTTP requests sent to `RequestExecutor`. But there is still code that does not support cancellation. For example, the API discovery, which we can look into later if needed.
