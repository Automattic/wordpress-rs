package rs.wordpress.api.kotlin

import uniffi.wp_api.UserListParams
import uniffi.wp_api.UserWithEditContext
import uniffi.wp_api.WpApiHelper
import uniffi.wp_api.WpContext
import uniffi.wp_api.parseListUsersResponseWithEditContext

class WPUsersEndpoint(
    private val requestHandler: WpRequestHandler,
    private val apiHelper: WpApiHelper
) : UsersEndpoint {
    override fun listWithEditContext(params: UserListParams?): WpRequestResult<List<UserWithEditContext>> =
        requestHandler.execute(
            request = apiHelper.listUsersRequest(
                context = WpContext.EDIT,
                params = params
            ),
            parser = ::parseListUsersResponseWithEditContext
        )
}
