package rs.wordpress.cache.kotlin

import uniffi.wp_mobile.FullEntityPostTypeDetailsWithEditContext
import uniffi.wp_mobile.PostTypeService

/**
 * Create an observable post type collection with edit context.
 *
 * Post types define what content types are available on a WordPress site (e.g., 'post', 'page',
 * custom post types). They're configuration data that rarely changes - typically only when
 * plugins are activated or deactivated.
 *
 * Unlike posts, post types don't support pagination - all types are returned in a single fetch.
 *
 * The collection provides:
 * - `loadData()`: Load all cached post types from the database
 * - Observable notifications when post types change
 *
 * To fetch post types from the network, use `PostTypeService.syncPostTypes()`.
 *
 * Example:
 * ```
 * class MyViewModel : ViewModel() {
 *     private val postTypeCollection = postTypeService.getObservablePostTypeCollectionWithEditContext()
 *
 *     init {
 *         postTypeCollection.addObserver {
 *             viewModelScope.launch {
 *                 val postTypes = postTypeCollection.loadData()
 *                 // Update UI with post types
 *             }
 *         }
 *
 *         // Initial fetch
 *         viewModelScope.launch {
 *             postTypeService.syncPostTypes()
 *         }
 *     }
 *
 *     override fun onCleared() {
 *         super.onCleared()
 *         postTypeCollection.close()
 *     }
 * }
 * ```
 *
 * @return Observable collection that notifies on database changes
 */
fun PostTypeService.getObservablePostTypeCollectionWithEditContext(): ObservableCollection<FullEntityPostTypeDetailsWithEditContext> {
    val collection = this.createPostTypeCollectionWithEditContext()
    return createObservableCollection(
        loadData = collection::loadData,
        isRelevantUpdate = collection::isRelevantUpdate
    )
}
