// Expose necessary Rust APIs as public API to the Swift package's consumers.
//
// We could export all of them using `@_exported import`, but that probably puts
// us in a position where we need to make major releases due to Rust code changes.

#if canImport(WordPressAPIInternal)

import WordPressAPIInternal

public typealias WpApiError = WordPressAPIInternal.WpApiError
public typealias ParsedUrl = WordPressAPIInternal.ParsedUrl

// MARK: - Login

public typealias WpApiApplicationPasswordDetails = WordPressAPIInternal.WpApiApplicationPasswordDetails
public typealias WpAuthentication = WordPressAPIInternal.WpAuthentication
public typealias UrlDiscoveryError = WordPressAPIInternal.UrlDiscoveryError
public typealias UrlDiscoverySuccess = WordPressAPIInternal.UrlDiscoverySuccess
public typealias UrlDiscoveryAttemptError = WordPressAPIInternal.UrlDiscoveryAttemptError

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
public typealias UsersRequestExecutor = WordPressAPIInternal.UsersRequestExecutor

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
public typealias PluginsRequestExecutor = WordPressAPIInternal.PluginsRequestExecutor

// MARK: – Application Passwords

public typealias SparseApplicationPassword = WordPressAPIInternal.SparseApplicationPassword
public typealias ApplicationPasswordWithEditContext = WordPressAPIInternal.ApplicationPasswordWithEditContext
public typealias ApplicationPasswordWithViewContext = WordPressAPIInternal.ApplicationPasswordWithViewContext
public typealias ApplicationPasswordWithEmbedContext = WordPressAPIInternal.ApplicationPasswordWithEmbedContext

// MARK: - Site Health Checks
public typealias SiteHealthTest = WordPressAPIInternal.WpSiteHealthTest
public typealias SiteHealthDirectorySizes = WordPressAPIInternal.WpSiteHealthDirectorySizes

// MARK: – Post Types
public typealias PostType = WordPressAPIInternal.PostType
public typealias SparsePostType = WordPressAPIInternal.SparsePostTypeDetails
public typealias PostTypeWithEditContext = WordPressAPIInternal.PostTypeDetailsWithEditContext
public typealias PostTypeWithViewContext = WordPressAPIInternal.PostTypeDetailsWithViewContext
public typealias PostTypeWithEmbedContext = WordPressAPIInternal.PostTypeDetailsWithEmbedContext
public typealias PostTypeDetailsWithEditContext = WordPressAPIInternal.PostTypeDetailsWithEditContext
public typealias PostTypeDetailsWithViewContext = WordPressAPIInternal.PostTypeDetailsWithViewContext
public typealias PostTypeDetailsWithEmbedContext = WordPressAPIInternal.PostTypeDetailsWithEmbedContext

// MARK: – Site Settings
public typealias SparseSiteSettings = WordPressAPIInternal.SparseSiteSettings
public typealias SiteSettingsWithEditContext = WordPressAPIInternal.SiteSettingsWithEditContext
public typealias SiteSettingsWithViewContext = WordPressAPIInternal.SiteSettingsWithViewContext
public typealias SiteSettingsWithEmbedContext = WordPressAPIInternal.SiteSettingsWithEmbedContext

#endif
