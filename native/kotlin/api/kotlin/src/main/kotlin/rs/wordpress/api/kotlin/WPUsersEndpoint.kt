package rs.wordpress.api.kotlin

import uniffi.wp_api.SparseUser
import uniffi.wp_api.SparseUserField
import uniffi.wp_api.UserListParams
import uniffi.wp_api.UserWithEditContext
import uniffi.wp_api.UserWithEmbedContext
import uniffi.wp_api.UserWithViewContext
import uniffi.wp_api.WpApiHelper
import uniffi.wp_api.WpContext
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.parseFilterUsersResponse
import uniffi.wp_api.parseListUsersResponseWithEditContext
import uniffi.wp_api.parseListUsersResponseWithEmbedContext
import uniffi.wp_api.parseListUsersResponseWithViewContext

class WPUsersEndpoint(
    private val requestHandler: WpRequestHandler,
    private val apiHelper: WpApiHelper
) : UsersEndpoint {
    override suspend fun listWithEditContext(params: UserListParams?): WpRequestResult<List<UserWithEditContext>> =
        listUsers(WpContext.EDIT, params, ::parseListUsersResponseWithEditContext)

    override suspend fun listWithEmbedContext(params: UserListParams?): WpRequestResult<List<UserWithEmbedContext>> =
        listUsers(WpContext.EMBED, params, ::parseListUsersResponseWithEmbedContext)

    override suspend fun listWithViewContext(params: UserListParams?): WpRequestResult<List<UserWithViewContext>> =
        listUsers(WpContext.VIEW, params, ::parseListUsersResponseWithViewContext)

    override suspend fun filterListUsers(
        context: WpContext,
        params: UserListParams?,
        fields: List<SparseUserField>
    ): WpRequestResult<List<SparseUser>> =
        requestHandler.execute(
            request = apiHelper.filterListUsersRequest(context, params, fields),
            ::parseFilterUsersResponse
        )

    private suspend fun <T> listUsers(
        context: WpContext,
        params: UserListParams?,
        parser: (response: WpNetworkResponse) -> T
    ): WpRequestResult<T> =
        requestHandler.execute(
            request = apiHelper.listUsersRequest(context, params),
            parser
        )
}
