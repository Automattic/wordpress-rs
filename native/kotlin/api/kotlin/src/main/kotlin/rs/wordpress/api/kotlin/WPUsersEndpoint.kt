package rs.wordpress.api.kotlin

import uniffi.wp_api.SparseUser
import uniffi.wp_api.SparseUserField
import uniffi.wp_api.UserCreateParams
import uniffi.wp_api.UserDeleteParams
import uniffi.wp_api.UserDeleteResponse
import uniffi.wp_api.UserId
import uniffi.wp_api.UserListParams
import uniffi.wp_api.UserUpdateParams
import uniffi.wp_api.UserWithEditContext
import uniffi.wp_api.UserWithEmbedContext
import uniffi.wp_api.UserWithViewContext
import uniffi.wp_api.WpApiHelper
import uniffi.wp_api.WpContext
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.parseDeleteUserResponse
import uniffi.wp_api.parseFilterRetrieveUserResponse
import uniffi.wp_api.parseFilterUsersResponse
import uniffi.wp_api.parseListUsersResponseWithEditContext
import uniffi.wp_api.parseListUsersResponseWithEmbedContext
import uniffi.wp_api.parseListUsersResponseWithViewContext
import uniffi.wp_api.parseRetrieveUserResponseWithEditContext
import uniffi.wp_api.parseRetrieveUserResponseWithEmbedContext
import uniffi.wp_api.parseRetrieveUserResponseWithViewContext

class WpUsersEndpoint(
    private val requestHandler: WpRequestHandler,
    private val apiHelper: WpApiHelper
) : UsersEndpoint {
    override val list: UsersEndpointList
        get() = WpUsersEndpointList(requestHandler, apiHelper)
    override val retrieve: UsersEndpointRetrieve
        get() = WpUsersEndpointRetrieve(requestHandler, apiHelper)
    override val me: UsersEndpointRetrieveMe
        get() = WpUsersEndpointRetrieveMe(requestHandler, apiHelper)
    override val create: UsersEndpointCreate
        get() = WpUsersEndpointCreate(requestHandler, apiHelper)
    override val update: UsersEndpointUpdate
        get() = WpUsersEndpointUpdate(requestHandler, apiHelper)
    override val delete: UsersEndpointDelete
        get() = WpUsersEndpointDelete(requestHandler, apiHelper)
}

private class WpUsersEndpointList(
    private val requestHandler: WpRequestHandler,
    private val apiHelper: WpApiHelper
) : UsersEndpointList {
    override suspend fun withEditContext(params: UserListParams?): WpRequestResult<List<UserWithEditContext>> =
        listUsers(WpContext.EDIT, params, ::parseListUsersResponseWithEditContext)

    override suspend fun withEmbedContext(params: UserListParams?): WpRequestResult<List<UserWithEmbedContext>> =
        listUsers(WpContext.EMBED, params, ::parseListUsersResponseWithEmbedContext)

    override suspend fun withViewContext(params: UserListParams?): WpRequestResult<List<UserWithViewContext>> =
        listUsers(WpContext.VIEW, params, ::parseListUsersResponseWithViewContext)

    override suspend fun filter(
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

private class WpUsersEndpointRetrieve(
    private val requestHandler: WpRequestHandler,
    private val apiHelper: WpApiHelper
) : UsersEndpointRetrieve {
    override suspend fun withEditContext(userId: UserId): WpRequestResult<UserWithEditContext> =
        retrieveUser(userId, WpContext.EDIT, ::parseRetrieveUserResponseWithEditContext)

    override suspend fun withEmbedContext(userId: UserId): WpRequestResult<UserWithEmbedContext> =
        retrieveUser(userId, WpContext.EMBED, ::parseRetrieveUserResponseWithEmbedContext)

    override suspend fun withViewContext(userId: UserId): WpRequestResult<UserWithViewContext> =
        retrieveUser(userId, WpContext.VIEW, ::parseRetrieveUserResponseWithViewContext)

    override suspend fun filter(
        userId: UserId,
        context: WpContext,
        fields: List<SparseUserField>
    ): WpRequestResult<SparseUser> =
        requestHandler.execute(
            request = apiHelper.filterRetrieveUserRequest(userId, context, fields),
            ::parseFilterRetrieveUserResponse
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
}

private class WpUsersEndpointRetrieveMe(
    private val requestHandler: WpRequestHandler,
    private val apiHelper: WpApiHelper
) : UsersEndpointRetrieveMe {
    override suspend fun withEditContext(): WpRequestResult<UserWithEditContext> =
        retrieveMe(WpContext.EDIT, ::parseRetrieveUserResponseWithEditContext)

    override suspend fun withEmbedContext(): WpRequestResult<UserWithEmbedContext> =
        retrieveMe(WpContext.EMBED, ::parseRetrieveUserResponseWithEmbedContext)

    override suspend fun withViewContext(): WpRequestResult<UserWithViewContext> =
        retrieveMe(WpContext.VIEW, ::parseRetrieveUserResponseWithViewContext)

    override suspend fun filter(
        context: WpContext,
        fields: List<SparseUserField>
    ): WpRequestResult<SparseUser> =
        requestHandler.execute(
            request = apiHelper.filterRetrieveCurrentUserRequest(context, fields),
            ::parseFilterRetrieveUserResponse
        )

    private suspend fun <T> retrieveMe(
        context: WpContext,
        parser: (response: WpNetworkResponse) -> T
    ): WpRequestResult<T> =
        requestHandler.execute(
            request = apiHelper.retrieveCurrentUserRequest(context),
            parser
        )
}

private class WpUsersEndpointCreate(
    private val requestHandler: WpRequestHandler,
    private val apiHelper: WpApiHelper
) : UsersEndpointCreate {
    override suspend fun new(params: UserCreateParams): WpRequestResult<UserWithEditContext> =
        requestHandler.execute(
            request = apiHelper.createUserRequest(params),
            ::parseRetrieveUserResponseWithEditContext
        )
}

private class WpUsersEndpointUpdate(
    private val requestHandler: WpRequestHandler,
    private val apiHelper: WpApiHelper
) : UsersEndpointUpdate {
    override suspend fun withId(
        userId: UserId,
        params: UserUpdateParams
    ): WpRequestResult<UserWithEditContext> =
        requestHandler.execute(
            request = apiHelper.updateUserRequest(userId, params),
            ::parseRetrieveUserResponseWithEditContext
        )

    override suspend fun current(params: UserUpdateParams): WpRequestResult<UserWithEditContext> =
        requestHandler.execute(
            request = apiHelper.updateCurrentUserRequest(params),
            ::parseRetrieveUserResponseWithEditContext
        )
}

private class WpUsersEndpointDelete(
    private val requestHandler: WpRequestHandler,
    private val apiHelper: WpApiHelper
) : UsersEndpointDelete {
    override suspend fun withId(
        userId: UserId,
        params: UserDeleteParams
    ): WpRequestResult<UserDeleteResponse> =
        requestHandler.execute(
            request = apiHelper.deleteUserRequest(userId, params),
            ::parseDeleteUserResponse
        )

    override suspend fun current(params: UserDeleteParams): WpRequestResult<UserDeleteResponse> =
        requestHandler.execute(
            request = apiHelper.deleteCurrentUserRequest(params),
            ::parseDeleteUserResponse
        )
}
