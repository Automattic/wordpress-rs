package rs.wordpress.api.kotlin

import uniffi.wp_api.UserId
import uniffi.wp_api.WpErrorCode

const val FIRST_USER_ID: UserId = 1
const val SECOND_USER_ID: UserId = 2
const val FIRST_USER_EMAIL = "test@example.com"
const val SECOND_USER_EMAIL = "themeshaperwp+demos@gmail.com"
const val SECOND_USER_SLUG = "themedemos"
const val NUMBER_OF_USERS = 4
const val NUMBER_OF_PLUGINS = 4
const val HELLO_DOLLY_PLUGIN_SLUG = "hello-dolly/hello"
const val WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS = "classic-widgets"

fun <T> WpRequestResult<T>.assertSuccess() {
    assert(this is WpRequestResult.WpRequestSuccess)
}

fun <T> WpRequestResult<T>.assertSuccessAndRetrieveData(): T {
    assert(this is WpRequestResult.WpRequestSuccess)
    return (this as WpRequestResult.WpRequestSuccess).data
}

fun <T> WpRequestResult<T>.wpErrorCode(): WpErrorCode {
    assert(this is WpRequestResult.WpError)
    return (this as WpRequestResult.WpError).errorCode
}
