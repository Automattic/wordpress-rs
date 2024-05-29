package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext
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
import uniffi.wp_api.UsersRequestBuilder
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

internal class WpUsersEndpoint(
    usersRequestBuilder: UsersRequestBuilder,
    networkHandler: NetworkHandler,
    dispatcher: CoroutineDispatcher
) : UsersEndpoint {
    private val requestHandler = WpRequestHandler(networkHandler, dispatcher)

    override val list: UsersEndpointList by lazy {
        WpUsersEndpointList(requestHandler, usersRequestBuilder, dispatcher)
    }
    override val retrieve: UsersEndpointRetrieve by lazy {
        WpUsersEndpointRetrieve(requestHandler, usersRequestBuilder, dispatcher)
    }
    override val me: UsersEndpointRetrieveMe by lazy {
        WpUsersEndpointRetrieveMe(requestHandler, usersRequestBuilder, dispatcher)
    }
    override val create: UsersEndpointCreate by lazy {
        WpUsersEndpointCreate(requestHandler, usersRequestBuilder, dispatcher)
    }
    override val update: UsersEndpointUpdate by lazy {
        WpUsersEndpointUpdate(requestHandler, usersRequestBuilder, dispatcher)
    }
    override val delete: UsersEndpointDelete by lazy {
        WpUsersEndpointDelete(requestHandler, usersRequestBuilder, dispatcher)
    }
}

private class WpUsersEndpointList(
    private val requestHandler: WpRequestHandler,
    private val usersRequestBuilder: UsersRequestBuilder,
    private val dispatcher: CoroutineDispatcher
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
    ): WpRequestResult<List<SparseUser>> = withContext(dispatcher) {
        requestHandler.execute(
            request = usersRequestBuilder.filterList(context, params, fields),
            ::parseFilterUsersResponse
        )
    }

    private suspend fun <T> listUsers(
        context: WpContext,
        params: UserListParams?,
        parser: (response: WpNetworkResponse) -> T
    ): WpRequestResult<T> = withContext(dispatcher) {
        requestHandler.execute(
            request = usersRequestBuilder.list(context, params),
            parser
        )
    }
}

private class WpUsersEndpointRetrieve(
    private val requestHandler: WpRequestHandler,
    private val usersRequestBuilder: UsersRequestBuilder,
    private val dispatcher: CoroutineDispatcher
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
    ): WpRequestResult<SparseUser> = withContext(dispatcher) {
        requestHandler.execute(
            request = usersRequestBuilder.filterRetrieve(userId, context, fields),
            ::parseFilterRetrieveUserResponse
        )
    }

    private suspend fun <T> retrieveUser(
        userId: UserId,
        context: WpContext,
        parser: (response: WpNetworkResponse) -> T
    ): WpRequestResult<T> = withContext(dispatcher) {
        requestHandler.execute(
            request = usersRequestBuilder.retrieve(userId, context),
            parser
        )
    }
}

private class WpUsersEndpointRetrieveMe(
    private val requestHandler: WpRequestHandler,
    private val usersRequestBuilder: UsersRequestBuilder,
    private val dispatcher: CoroutineDispatcher
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
    ): WpRequestResult<SparseUser> = withContext(dispatcher) {
        requestHandler.execute(
            request = usersRequestBuilder.filterRetrieveMe(context, fields),
            ::parseFilterRetrieveUserResponse
        )
    }

    private suspend fun <T> retrieveMe(
        context: WpContext,
        parser: (response: WpNetworkResponse) -> T
    ): WpRequestResult<T> = withContext(dispatcher) {
        requestHandler.execute(
            request = usersRequestBuilder.retrieveMe(context),
            parser
        )
    }
}

private class WpUsersEndpointCreate(
    private val requestHandler: WpRequestHandler,
    private val usersRequestBuilder: UsersRequestBuilder,
    private val dispatcher: CoroutineDispatcher
) : UsersEndpointCreate {
    override suspend fun new(params: UserCreateParams): WpRequestResult<UserWithEditContext> =
        withContext(dispatcher) {
            requestHandler.execute(
                request = usersRequestBuilder.create(params),
                ::parseRetrieveUserResponseWithEditContext
            )
        }
}

private class WpUsersEndpointUpdate(
    private val requestHandler: WpRequestHandler,
    private val usersRequestBuilder: UsersRequestBuilder,
    private val dispatcher: CoroutineDispatcher
) : UsersEndpointUpdate {
    override suspend fun withId(
        userId: UserId,
        params: UserUpdateParams
    ): WpRequestResult<UserWithEditContext> = withContext(dispatcher) {
        requestHandler.execute(
            request = usersRequestBuilder.update(userId, params),
            ::parseRetrieveUserResponseWithEditContext
        )
    }

    override suspend fun me(params: UserUpdateParams): WpRequestResult<UserWithEditContext> =
        withContext(dispatcher) {
            requestHandler.execute(
                request = usersRequestBuilder.updateMe(params),
                ::parseRetrieveUserResponseWithEditContext
            )
        }
}

private class WpUsersEndpointDelete(
    private val requestHandler: WpRequestHandler,
    private val usersRequestBuilder: UsersRequestBuilder,
    private val dispatcher: CoroutineDispatcher
) : UsersEndpointDelete {
    override suspend fun withId(
        userId: UserId,
        params: UserDeleteParams
    ): WpRequestResult<UserDeleteResponse> = withContext(dispatcher) {
        requestHandler.execute(
            request = usersRequestBuilder.delete(userId, params),
            ::parseDeleteUserResponse
        )
    }

    override suspend fun me(params: UserDeleteParams): WpRequestResult<UserDeleteResponse> =
        withContext(dispatcher) {
            requestHandler.execute(
                request = usersRequestBuilder.deleteMe(params),
                ::parseDeleteUserResponse
            )
        }
}
