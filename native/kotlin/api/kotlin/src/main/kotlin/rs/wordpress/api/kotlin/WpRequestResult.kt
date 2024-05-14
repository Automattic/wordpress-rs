package rs.wordpress.api.kotlin

import uniffi.wp_api.UnrecognizedWpRestError
import uniffi.wp_api.WpRestError

sealed class WpRequestResult<T>
class RecognizedRestError<T>(val error: WpRestError) : WpRequestResult<T>()
class UnrecognizedRestError<T>(val error: UnrecognizedWpRestError) : WpRequestResult<T>()
class WpRequestInternalException<T>(val exception: Exception) : WpRequestResult<T>()
class WpRequestSuccess<T>(val value: T) : WpRequestResult<T>()
