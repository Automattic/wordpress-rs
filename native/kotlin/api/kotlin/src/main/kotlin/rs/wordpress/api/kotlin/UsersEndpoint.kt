package rs.wordpress.api.kotlin

import uniffi.wp_api.UserListParams
import uniffi.wp_api.UserWithEditContext

interface UsersEndpoint {
    suspend fun listWithEditContext(params: UserListParams?): WpRequestResult<List<UserWithEditContext>>
}
