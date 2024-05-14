package rs.wordpress.api.kotlin

import uniffi.wp_api.SparseUser
import uniffi.wp_api.SparseUserField
import uniffi.wp_api.UserId
import uniffi.wp_api.UserListParams
import uniffi.wp_api.UserWithEditContext
import uniffi.wp_api.UserWithEmbedContext
import uniffi.wp_api.UserWithViewContext
import uniffi.wp_api.WpContext

interface UsersEndpoint {
    val list: UsersEndpointList
    val retrieve: UsersEndpointRetrieve
    val retrieveCurrent: UsersEndpointRetrieveCurrent
}

interface UsersEndpointList {
    suspend fun withEditContext(params: UserListParams?): WpRequestResult<List<UserWithEditContext>>
    suspend fun withEmbedContext(params: UserListParams?): WpRequestResult<List<UserWithEmbedContext>>
    suspend fun withViewContext(params: UserListParams?): WpRequestResult<List<UserWithViewContext>>
    suspend fun filter(
        context: WpContext,
        params: UserListParams?,
        fields: List<SparseUserField>
    ): WpRequestResult<List<SparseUser>>
}

interface UsersEndpointRetrieve {
    suspend fun withEditContext(userId: UserId): WpRequestResult<UserWithEditContext>
    suspend fun withEmbedContext(userId: UserId): WpRequestResult<UserWithEmbedContext>
    suspend fun withViewContext(userId: UserId): WpRequestResult<UserWithViewContext>
    suspend fun filter(
        userId: UserId,
        context: WpContext,
        fields: List<SparseUserField>
    ): WpRequestResult<SparseUser>
}

interface UsersEndpointRetrieveCurrent {
    suspend fun withEditContext(): WpRequestResult<UserWithEditContext>
    suspend fun withEmbedContext(): WpRequestResult<UserWithEmbedContext>
    suspend fun userWithViewContext(): WpRequestResult<UserWithViewContext>
    suspend fun filter(
        context: WpContext,
        fields: List<SparseUserField>
    ): WpRequestResult<SparseUser>
}
