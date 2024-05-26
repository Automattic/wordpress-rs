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

// MARK: - Plugins

public typealias SparsePlugin = WordPressAPIInternal.SparsePlugin
public typealias PluginWithViewContext = WordPressAPIInternal.PluginWithViewContext
public typealias PluginWithEditContext = WordPressAPIInternal.PluginWithEditContext
public typealias PluginWithEmbedContext = WordPressAPIInternal.PluginWithEmbedContext
