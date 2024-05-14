package rs.wordpress.api.kotlin

import uniffi.wp_api.UserListParams
import uniffi.wp_api.UserWithEditContext
import uniffi.wp_api.UserWithEmbedContext
import uniffi.wp_api.UserWithViewContext

interface UsersEndpoint {
    suspend fun listWithEditContext(params: UserListParams?): WpRequestResult<List<UserWithEditContext>>
    suspend fun listWithEmbedContext(params: UserListParams?): WpRequestResult<List<UserWithEmbedContext>>
    suspend fun listWithViewContext(params: UserListParams?): WpRequestResult<List<UserWithViewContext>>
}
