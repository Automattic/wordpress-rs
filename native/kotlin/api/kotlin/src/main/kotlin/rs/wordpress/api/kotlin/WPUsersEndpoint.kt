package rs.wordpress.api.kotlin

import uniffi.wp_api.UserListParams
import uniffi.wp_api.UserWithEditContext
import uniffi.wp_api.UserWithEmbedContext
import uniffi.wp_api.UserWithViewContext
import uniffi.wp_api.WpApiHelper
import uniffi.wp_api.WpContext
import uniffi.wp_api.parseListUsersResponseWithEditContext
import uniffi.wp_api.parseListUsersResponseWithEmbedContext
import uniffi.wp_api.parseListUsersResponseWithViewContext

class WPUsersEndpoint(
    private val requestHandler: WpRequestHandler,
    private val apiHelper: WpApiHelper
) : UsersEndpoint {
    override suspend fun listWithEditContext(params: UserListParams?): WpRequestResult<List<UserWithEditContext>> =
        requestHandler.execute(
            request = apiHelper.listUsersRequest(
                context = WpContext.EDIT,
                params = params
            ),
            parser = ::parseListUsersResponseWithEditContext
        )

    override suspend fun listWithEmbedContext(params: UserListParams?): WpRequestResult<List<UserWithEmbedContext>> =
        requestHandler.execute(
            request = apiHelper.listUsersRequest(
                context = WpContext.EMBED,
                params = params
            ),
            parser = ::parseListUsersResponseWithEmbedContext
        )

    override suspend fun listWithViewContext(params: UserListParams?): WpRequestResult<List<UserWithViewContext>> =
        requestHandler.execute(
            request = apiHelper.listUsersRequest(
                context = WpContext.VIEW,
                params = params
            ),
            parser = ::parseListUsersResponseWithViewContext
        )
}
