package rs.wordpress.api.kotlin

import uniffi.wp_api.SparseUser
import uniffi.wp_api.SparseUserField
import uniffi.wp_api.UserId
import uniffi.wp_api.UserListParams
import uniffi.wp_api.UserWithEditContext
import uniffi.wp_api.UserWithEmbedContext
import uniffi.wp_api.UserWithViewContext
import uniffi.wp_api.WpApiHelper
import uniffi.wp_api.WpContext
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.parseFilterRetrieveUserResponse
import uniffi.wp_api.parseFilterUsersResponse
import uniffi.wp_api.parseListUsersResponseWithEditContext
import uniffi.wp_api.parseListUsersResponseWithEmbedContext
import uniffi.wp_api.parseListUsersResponseWithViewContext
import uniffi.wp_api.parseRetrieveUserResponseWithEditContext
import uniffi.wp_api.parseRetrieveUserResponseWithEmbedContext
import uniffi.wp_api.parseRetrieveUserResponseWithViewContext

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

    override suspend fun retrieveWithEditContext(userId: UserId): WpRequestResult<UserWithEditContext> =
        retrieveUser(userId, WpContext.EDIT, ::parseRetrieveUserResponseWithEditContext)

    override suspend fun retrieveWithEmbedContext(userId: UserId): WpRequestResult<UserWithEmbedContext> =
        retrieveUser(userId, WpContext.EMBED, ::parseRetrieveUserResponseWithEmbedContext)

    override suspend fun retrieveWithViewContext(userId: UserId): WpRequestResult<UserWithViewContext> =
        retrieveUser(userId, WpContext.VIEW, ::parseRetrieveUserResponseWithViewContext)

    override suspend fun filterRetrieveUser(
        userId: UserId,
        context: WpContext,
        fields: List<SparseUserField>
    ): WpRequestResult<SparseUser> =
        requestHandler.execute(
            request = apiHelper.filterRetrieveUserRequest(userId, context, fields),
            ::parseFilterRetrieveUserResponse
        )

    override suspend fun retrieveCurrentUserWithEditContext(): WpRequestResult<UserWithEditContext> =
        retrieveCurrentUser(WpContext.EDIT, ::parseRetrieveUserResponseWithEditContext)

    override suspend fun retrieveCurrentUserWithEmbedContext(): WpRequestResult<UserWithEmbedContext> =
        retrieveCurrentUser(WpContext.EMBED, ::parseRetrieveUserResponseWithEmbedContext)

    override suspend fun retrieveCurrentUserWithViewContext(): WpRequestResult<UserWithViewContext> =
        retrieveCurrentUser(WpContext.VIEW, ::parseRetrieveUserResponseWithViewContext)

    override suspend fun filterRetrieveCurrentUser(
        context: WpContext,
        fields: List<SparseUserField>
    ): WpRequestResult<SparseUser> =
        requestHandler.execute(
            request = apiHelper.filterRetrieveCurrentUserRequest(context, fields),
            ::parseFilterRetrieveUserResponse
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

    private suspend fun <T> retrieveUser(
        userId: UserId,
        context: WpContext,
        parser: (response: WpNetworkResponse) -> T
    ): WpRequestResult<T> =
        requestHandler.execute(
            request = apiHelper.retrieveUserRequest(userId, context),
            parser
        )

    private suspend fun <T> retrieveCurrentUser(
        context: WpContext,
        parser: (response: WpNetworkResponse) -> T
    ): WpRequestResult<T> =
        requestHandler.execute(
            request = apiHelper.retrieveCurrentUserRequest(context),
            parser
        )
}
