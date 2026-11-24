import Foundation
import Testing
import WordPressAPI
import WordPressApiCache

#if canImport(Combine)
import Combine
#endif

/// Hears what becomes of a like the way an app does, through `WordPressAPI` and
/// `WordPressApiCache` only. None of these calls reach a server.
struct QueuedActionEventTests {

    private func openCache() throws -> WordPressApiCache {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("queued-actions-\(UUID().uuidString).sqlite")
        let cache = try WordPressApiCache(path: url.path)
        _ = try cache.performMigrations()
        return cache
    }

    private let post = ReaderPostSaveIdentity(
        id: 42,
        siteId: 7001,
        feedId: nil,
        feedItemId: nil,
        isExternal: false,
        url: nil
    )

    @Test
    func aRefusedLikeHasAMessageForTheUserAndAWaitingOneHasNone() {
        let refused = QueuedActionEvent(
            userId: 1,
            action: .postLike(post: post),
            on: true,
            state: .refused(error: .MissingIds)
        )
        #expect(refused.userMessage(languages: ["en-US"]) == "Your like was not added to this post.")
        let waiting = QueuedActionEvent(userId: 1, action: .postLike(post: post), on: true, state: .waiting)
        #expect(waiting.userMessage(languages: ["en-US"]) == nil)
        let tag = QueuedActionEvent(
            userId: 1,
            action: .tagFollow(slug: "art"),
            on: true,
            state: .refused(error: .MissingIds)
        )
        #expect(tag.userMessage == nil)
    }

    /// Neither the cache's own listener nor an observer keeps the cache alive.
    @Test
    func aCacheWithAnObserverIsFreed() throws {
        weak var freed: WordPressApiCache?
        var observer: NSObjectProtocol?
        do {
            let cache = try openCache()
            freed = cache
            observer = cache.addQueuedActionObserver { _ in }
        }
        #expect(freed == nil)
        if let observer {
            NotificationCenter.default.removeObserver(observer)
        }
    }

    // Only where `QueuedActionsOfflineURLProtocol` is sure to catch a request with a body: on the
    // watchOS simulator it doesn't, and the like would reach WordPress.com.
    #if os(macOS) || os(iOS)
    /// A like made offline in the Reader is heard as waiting by every kind of observer of its
    /// cache, and by none of another cache.
    @Test(.timeLimit(.minutes(1)))
    func aLikeMadeOfflineIsHeardAsWaitingByEveryObserverOfItsCacheOnly() async throws {
        let cache = try openCache()
        let other = try openCache()
        let heard = Heard()

        let observer = cache.addQueuedActionObserver { heard.append("closure", $0) }
        defer { NotificationCenter.default.removeObserver(observer) }
        let otherObserver = other.addQueuedActionObserver { heard.append("other cache", $0) }
        defer { NotificationCenter.default.removeObserver(otherObserver) }

        #if canImport(Combine)
        let subscription = cache.queuedActionEventsPublisher().sink { heard.append("publisher", $0) }
        defer { subscription.cancel() }
        #endif

        var tokens: [Any] = []
        if #available(macOS 26, iOS 26, tvOS 26, watchOS 26, visionOS 26, *) {
            tokens.append(
                NotificationCenter.default.addObserver(of: cache, for: .queuedActionChanged) { message in
                    heard.append("message", message.event)
                }
            )
            tokens.append(
                NotificationCenter.default.addObserver(of: other, for: .queuedActionChanged) { message in
                    heard.append("other cache message", message.event)
                }
            )
        }
        defer {
            if #available(macOS 26, iOS 26, tvOS 26, watchOS 26, visionOS 26, *) {
                for case let token as NotificationCenter.ObservationToken in tokens {
                    NotificationCenter.default.removeObserver(token)
                }
            }
        }

        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [QueuedActionsOfflineURLProtocol.self]
        let offline = WPComApiClient(urlSession: URLSession(configuration: configuration), authentication: .none)
        let reader = try offline.makeReaderService(cache: cache, owner: .account(userId: 1), savedSource: .ios)
        _ = try await reader.setPostLiked(post: post, liked: true, pageView: .dontCount)

        var expected = ["closure"]
        #if canImport(Combine)
        expected.append("publisher")
        #endif
        if #available(macOS 26, iOS 26, tvOS 26, watchOS 26, visionOS 26, *) {
            expected.append("message")
        }
        // The typed message's observer is called asynchronously.
        try await heard.waitFor(count: expected.count)
        // Give anything another cache's observer would hear time to arrive.
        try await Task.sleep(for: .milliseconds(200))

        let events = heard.events
        #expect(events.map(\.observer).sorted() == expected.sorted())
        for (_, event) in events {
            #expect(event.userId == 1)
            #expect(event.on)
            guard case .postLike(let liked) = event.action else {
                Issue.record("Expected a post like, heard \(event.action)")
                continue
            }
            #expect(liked == post)
            guard case .waiting = event.state else {
                Issue.record("Expected a waiting like, heard \(event.state)")
                continue
            }
        }
    }
    #endif

    #if canImport(Darwin)
    /// A typed message posted for a cache reaches the classic observers of that cache.
    @Test(.timeLimit(.minutes(1)))
    func aTypedMessageIsHeardByTheClassicObserversOfItsSubject() async throws {
        guard #available(macOS 26, iOS 26, tvOS 26, watchOS 26, visionOS 26, *) else { return }
        let cache = try openCache()
        let other = try openCache()
        let heard = Heard()
        let observer = cache.addQueuedActionObserver { heard.append("closure", $0) }
        defer { NotificationCenter.default.removeObserver(observer) }
        let otherObserver = other.addQueuedActionObserver { heard.append("other cache", $0) }
        defer { NotificationCenter.default.removeObserver(otherObserver) }

        let event = QueuedActionEvent(userId: 1, action: .siteBlock(siteId: 7001), on: true, state: .sent)
        NotificationCenter.default.post(WordPressApiCache.QueuedActionChangedMessage(event: event), subject: cache)

        try await heard.waitFor(count: 1)
        try await Task.sleep(for: .milliseconds(200))
        #expect(heard.events.map(\.observer) == ["closure"])
    }

    @Test(.timeLimit(.minutes(1)))
    func aCachesMessagesCanBeAwaited() async throws {
        guard #available(macOS 26, iOS 26, tvOS 26, watchOS 26, visionOS 26, *) else { return }
        let cache = try openCache()
        let messages = NotificationCenter.default.messages(of: cache, for: .queuedActionChanged)
        let first = Task { () -> QueuedActionEvent? in
            for await message in messages {
                return message.event
            }
            return nil
        }
        let event = QueuedActionEvent(userId: 1, action: .tagFollow(slug: "art"), on: false, state: .sent)
        // The sequence may start observing only once it's iterated, so post until it hears one.
        let poster = Task { () async throws -> Void in
            while !Task.isCancelled {
                NotificationCenter.default.post(
                    WordPressApiCache.QueuedActionChangedMessage(event: event),
                    subject: cache
                )
                try await Task.sleep(for: .milliseconds(50))
            }
        }
        defer { poster.cancel() }

        let heard = await first.value
        guard case .tagFollow(let slug) = heard?.action else {
            Issue.record("Expected a tag follow, heard \(String(describing: heard))")
            return
        }
        #expect(slug == "art")
        #expect(heard?.on == false)
    }
    #endif
}

/// The events each observer heard, from any thread.
private final class Heard: @unchecked Sendable {
    private let lock = NSLock()
    private var heard: [(observer: String, event: QueuedActionEvent)] = []

    var events: [(observer: String, event: QueuedActionEvent)] {
        lock.lock()
        defer { lock.unlock() }
        return heard
    }

    func append(_ observer: String, _ event: QueuedActionEvent) {
        lock.lock()
        defer { lock.unlock() }
        heard.append((observer, event))
    }

    /// Waits up to five seconds for `count` events.
    func waitFor(count: Int) async throws {
        for _ in 0..<100 where events.count < count {
            try await Task.sleep(for: .milliseconds(50))
        }
    }
}

#if os(macOS) || os(iOS)
/// Fails every request as a device without a connection does, so nothing reaches the network.
private final class QueuedActionsOfflineURLProtocol: URLProtocol {
    override static func canInit(with request: URLRequest) -> Bool { true }

    override static func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        client?.urlProtocol(self, didFailWithError: URLError(.notConnectedToInternet))
    }

    override func stopLoading() {}
}
#endif
