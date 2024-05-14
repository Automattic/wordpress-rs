package rs.wordpress.api.kotlin

import uniffi.wp_api.UnrecognizedWpRestError
import uniffi.wp_api.WpRestError

sealed class WpRequestResult<T>
class RecognizedRestError<T>(val error: WpRestError) : WpRequestResult<T>()
class UnrecognizedRestError<T>(val error: UnrecognizedWpRestError) : WpRequestResult<T>()
class WpRequestSuccess<T>(val data: T) : WpRequestResult<T>()
