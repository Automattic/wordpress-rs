package rs.wordpress.api.kotlin

import uniffi.wp_api.UnrecognizedWpRestError
import uniffi.wp_api.UserListParams
import uniffi.wp_api.UserWithEditContext
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpApiHelper
import uniffi.wp_api.WpContext
import uniffi.wp_api.WpRestError
import uniffi.wp_api.WpRestErrorWrapper
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
    private val networkHandler: NetworkHandler,
    private val apiHelper: WpApiHelper
) : UsersEndpoint {
    override fun listWithEditContext(params: UserListParams?): WpRequestResult<List<UserWithEditContext>> {
        val request = apiHelper.listUsersRequest(context = WpContext.EDIT, params = params)
        try {
            val response = networkHandler.request(request)
            val parsedResponse: List<UserWithEditContext> =
                parseListUsersResponseWithEditContext(response)
            return WpRequestSuccess(value = parsedResponse)
        } catch (restException: WpApiException.RestException) {
            return when(restException.restError) {
                is WpRestErrorWrapper.Recognized -> {
                    RecognizedRestError(error = restException.restError.v1)
                }
                is WpRestErrorWrapper.Unrecognized -> {
                    UnrecognizedRestError(error = restException.restError.v1)
                }
            }
        } catch (e: Exception) {
            return UncaughtException(exception = e)
        }
    }
}