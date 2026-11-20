import Foundation
import Testing

@testable import WordPressAPI
@testable import WordPressAPIInternal

/// A request from an API root that can't have a path, such as `data:,`, used to crash the app (#1781).
struct ApiUrlResolverTests {

    @Test(arguments: [
        "data:,",
        "about:blank",
        "mailto:someone@example.com",
        "data:,?rest_route=/",
        "about:blank?rest_route=/"
    ])
    func requestFromAnApiRootThatCannotHaveAPathFails(apiRoot: String) async throws {
        let api = try WordPressAPI(
            siteInfo: .selfHosted(
                siteUrl: ParsedUrl.parse(input: "https://example.com"),
                apiRoot: ParsedUrl.parse(input: apiRoot)
            ),
            authenticationProvider: .none(),
            // Fails any request it gets with `RequestExecutionFailed`.
            executor: HTTPStubs(),
            middlewarePipeline: .default,
            appNotifier: nil
        )

        await #expect(throws: WpApiError.SiteUrlParsingError(reason: "RelativeUrlWithCannotBeABaseBase")) {
            _ = try await api.users.retrieveMeWithEditContext()
        }
    }

    /// `ApiUrlResolver.resolve` can throw. An error that isn't a `ParseUrlError` used to crash the app.
    @Test
    func requestFromAResolverThatThrowsAnotherErrorFails() async throws {
        let client = UniffiWpApiClient(
            apiUrlResolver: ThrowingApiUrlResolver(),
            delegate: WpApiClientDelegate(
                authProvider: .none(),
                requestExecutor: HTTPStubs(),
                middlewarePipeline: .default,
                appNotifier: EmptyAppNotifier()
            )
        )

        await #expect(throws: WpApiError.SiteUrlParsingError(reason: "UnexpectedResolverError")) {
            _ = try await client.users().retrieveMeWithEditContext()
        }
    }
}

private struct AppResolverError: Error {}

private final class ThrowingApiUrlResolver: ApiUrlResolver {
    func resolve(namespace: String, endpointSegments: [String]) throws -> ParsedUrl {
        throw AppResolverError()
    }

    func routePath(namespace: String, endpointPath: String) -> String {
        "\(namespace)/\(endpointPath)"
    }
}
