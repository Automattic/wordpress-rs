import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

@Suite("SafeRequestExecutor")
struct SafeRequestExecutorTests {

    // Regression test for #1513: `sleep(millis:)` converted milliseconds to nanoseconds with the
    // wrong factor (`millis * 1_000` instead of `* 1_000_000`), so it slept 1000× too short and
    // `RetryAfterMiddleware` never actually honored a `Retry-After` backoff.
    @Test("sleep(millis:) waits for approximately the requested duration")
    func testSleepHonorsMillisecondDuration() async {
        let executor = WpRequestExecutor(urlSession: .shared)

        let requestedMillis: UInt64 = 200
        let clock = ContinuousClock()
        let start = clock.now
        await executor.sleep(millis: requestedMillis)
        let elapsed = start.duration(to: clock.now)

        // The old bug slept ~0.2 ms for a 200 ms request. `Task.sleep` waits *at least* the
        // requested duration, so a 150 ms lower bound sits comfortably above the buggy value and
        // below the target — catching the regression without flaking on scheduling jitter.
        #expect(elapsed >= .milliseconds(150))
    }
}
