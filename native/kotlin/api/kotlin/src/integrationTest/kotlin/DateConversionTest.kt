package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.assertDateIsConvertedFromNativeToRustCorrectly
import uniffi.wp_api.assertionExampleDateThatCanBeUsedToVerifyConversionBetweenRustAndNative
import java.text.SimpleDateFormat
import java.util.Date

// The date has to match the `EXAMPLE_DATE` from `wp_api/src/date.rs`
private const val EXAMPLE_DATE: String = "2020-08-14T15:00:00+02:00"

class DateConversionTest {
    @Test
    fun testDateIsConvertedCorrectlyFromKotlinToRust() = runTest {
        val parser = SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ssX")
        val date: Date = parser.parse(EXAMPLE_DATE)
        assertDateIsConvertedFromNativeToRustCorrectly(date)
    }

    @Test
    fun testDateIsConvertedCorrectlyBetweenKotlinAndRust() = runTest {
        val dateFromRust = assertionExampleDateThatCanBeUsedToVerifyConversionBetweenRustAndNative()
        assertDateIsConvertedFromNativeToRustCorrectly(dateFromRust)
    }
}