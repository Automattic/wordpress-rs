// Expose necessary Rust APIs as public API to the Swift package's consumers.
//
// We could export all of them using `@_exported import`, but that probably puts
// us in a position where we need to make major releases due to Rust code changes.

import WordPressAPIInternal

public typealias WpApiError = WordPressAPIInternal.WpApiError

// MARK: - Users

public typealias SparseUser = WordPressAPIInternal.SparseUser
public typealias UserWithViewContext = WordPressAPIInternal.UserWithViewContext
public typealias UserWithEditContext = WordPressAPIInternal.UserWithEditContext
public typealias UserWithEmbedContext = WordPressAPIInternal.UserWithEmbedContext
public typealias UserListParams = WordPressAPIInternal.UserListParams
public typealias UserUpdateParams = WordPressAPIInternal.UserUpdateParams
public typealias UserCreateParams = WordPressAPIInternal.UserCreateParams
public typealias UserDeleteParams = WordPressAPIInternal.UserDeleteParams
public typealias UserDeleteResponse = WordPressAPIInternal.UserDeleteResponse

// MARK: - Plugins

public typealias SparsePlugin = WordPressAPIInternal.SparsePlugin
public typealias PluginWithViewContext = WordPressAPIInternal.PluginWithViewContext
public typealias PluginWithEditContext = WordPressAPIInternal.PluginWithEditContext
public typealias PluginWithEmbedContext = WordPressAPIInternal.PluginWithEmbedContext
public typealias PluginSlug = WordPressAPIInternal.PluginSlug
public typealias PluginWpOrgDirectorySlug = WordPressAPIInternal.PluginWpOrgDirectorySlug
public typealias PluginListParams = WordPressAPIInternal.PluginListParams
public typealias PluginUpdateParams = WordPressAPIInternal.PluginUpdateParams
public typealias PluginCreateParams = WordPressAPIInternal.PluginCreateParams
public typealias PluginDeleteResponse = WordPressAPIInternal.PluginDeleteResponse
