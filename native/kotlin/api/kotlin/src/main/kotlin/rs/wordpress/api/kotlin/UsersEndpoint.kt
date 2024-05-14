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
    suspend fun listWithEditContext(params: UserListParams?): WpRequestResult<List<UserWithEditContext>>
    suspend fun listWithEmbedContext(params: UserListParams?): WpRequestResult<List<UserWithEmbedContext>>
    suspend fun listWithViewContext(params: UserListParams?): WpRequestResult<List<UserWithViewContext>>
    suspend fun filterListUsers(
        context: WpContext,
        params: UserListParams?,
        fields: List<SparseUserField>
    ): WpRequestResult<List<SparseUser>>
    suspend fun retrieveWithEditContext(userId: UserId): WpRequestResult<UserWithEditContext>
    suspend fun retrieveWithEmbedContext(userId: UserId): WpRequestResult<UserWithEmbedContext>
    suspend fun retrieveWithViewContext(userId: UserId): WpRequestResult<UserWithViewContext>
    suspend fun filterRetrieveUser(
        userId: UserId,
        context: WpContext,
        fields: List<SparseUserField>
    ): WpRequestResult<SparseUser>
    suspend fun retrieveCurrentUserWithEditContext(): WpRequestResult<UserWithEditContext>
    suspend fun retrieveCurrentUserWithEmbedContext(): WpRequestResult<UserWithEmbedContext>
    suspend fun retrieveCurrentUserWithViewContext(): WpRequestResult<UserWithViewContext>
    suspend fun filterRetrieveCurrentUser(
        context: WpContext,
        fields: List<SparseUserField>
    ): WpRequestResult<SparseUser>
}
