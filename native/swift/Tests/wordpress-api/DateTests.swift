import Foundation
import Testing
import WordPressAPIInternal

struct DateTests {
    var testDate: Date {
        // This is the time that's hard-coded in the Rust library.
        let str = "2020-08-14T15:00:00+02:00"
        return ISO8601DateFormatter().date(from: str)!
    }

    @Test
    func swiftToRust() {
        assertDateIsConvertedFromNativeToRustCorrectly(date: testDate)
    }

    @Test
    func rustToSwift() {
        let date = assertionExampleDateThatCanBeUsedToVerifyConversionBetweenRustAndNative()
        #expect(testDate == date)
    }

    @Test
    func roundtrip() {
        let date = assertionExampleDateThatCanBeUsedToVerifyConversionBetweenRustAndNative()
        assertDateIsConvertedFromNativeToRustCorrectly(date: date)
    }
}
