package rs.wordpress.cache.kotlin

import uniffi.wp_mobile.FullEntityPostTypeDetailsWithEditContext
import uniffi.wp_mobile.PostTypeFilter
import uniffi.wp_mobile.PostTypeService

/**
 * Create an observable post type collection with edit context.
 *
 * Post types define what content types are available on a WordPress site (e.g., 'post', 'page',
 * custom post types). They're configuration data that rarely changes - typically only when
 * plugins are activated or deactivated. Unlike posts, post types don't support pagination -
 * all types are returned in a single fetch.
 *
 * The collection provides:
 * - `loadData()`: Load all cached post types from the database
 * - Observable notifications when post types change
 *
 * By default, only viewable and UI-visible post types are returned
 * (those with `viewable = true` and `show_ui = true`).
 * For custom filtering, use [getObservablePostTypeCollectionWithEditContextFiltered].
 *
 * Example:
 * ```
 * class MyViewModel : ViewModel() {
 *     // Get viewable and UI-visible post types (default behavior)
 *     private val postTypeCollection = postTypeService.getObservablePostTypeCollectionWithEditContext()
 *
 *     init {
 *         postTypeCollection.addObserver {
 *             viewModelScope.launch {
 *                 val postTypes = postTypeCollection.loadData()
 *                 // Update UI with post types
 *             }
 *         }
 *     }
 * }
 * ```
 *
 * @return Observable collection that notifies on database changes
 */
fun PostTypeService.getObservablePostTypeCollectionWithEditContext():
    ObservableCollection<FullEntityPostTypeDetailsWithEditContext> {
    val collection = this.createPostTypeCollectionWithEditContext()
    return createObservableCollection(
        loadData = collection::loadData,
        isRelevantUpdate = collection::isRelevantUpdate
    )
}

/**
 * Create an observable post type collection with edit context using a custom filter.
 *
 * See [getObservablePostTypeCollectionWithEditContext] for general information.
 *
 * Example:
 * ```
 * // Get only hierarchical post types that are viewable and shown in UI
 * val filter = PostTypeFilter(viewable = true, showUi = true, hierarchical = true)
 * val collection = postTypeService.getObservablePostTypeCollectionWithEditContextFiltered(filter)
 * ```
 *
 * @param filter Filter criteria for post types (viewable, show_ui, hierarchical)
 * @return Observable collection that notifies on database changes
 */
fun PostTypeService.getObservablePostTypeCollectionWithEditContextFiltered(
    filter: PostTypeFilter
): ObservableCollection<FullEntityPostTypeDetailsWithEditContext> {
    val collection = this.createPostTypeCollectionWithEditContextFiltered(filter)
    return createObservableCollection(
        loadData = collection::loadData,
        isRelevantUpdate = collection::isRelevantUpdate
    )
}
