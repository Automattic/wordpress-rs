package rs.wordpress.api.kotlin

import uniffi.wp_api.UnrecognizedWpRestError
import uniffi.wp_api.UserListParams
import uniffi.wp_api.UserWithEditContext
import uniffi.wp_api.WpApiHelper
import uniffi.wp_api.WpContext
import uniffi.wp_api.WpRestError
import uniffi.wp_api.parseListUsersResponseWithEditContext

sealed class WpRequestResult<T>
class WpRequestSuccess<T>(val value: T) : WpRequestResult<T>()
class RecognizedRestError<T>(val error: WpRestError) : WpRequestResult<T>()
class UnrecognizedRestError<T>(val error: UnrecognizedWpRestError) : WpRequestResult<T>()
class UncaughtException<T>(val exception: Exception) : WpRequestResult<T>()

interface UsersEndpoint {
    fun listWithEditContext(params: UserListParams?): WpRequestResult<List<UserWithEditContext>>
}

class WPUsersEndpoint(
    private val requestHandler: WpRequestHandler,
    private val apiHelper: WpApiHelper
) : UsersEndpoint {
    override fun listWithEditContext(params: UserListParams?): WpRequestResult<List<UserWithEditContext>> =
        requestHandler.execute(
            request = apiHelper.listUsersRequest(
                context = WpContext.EDIT,
                params = params
            ), parser = ::parseListUsersResponseWithEditContext)
}