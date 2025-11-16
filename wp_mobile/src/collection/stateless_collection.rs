use wp_mobile_cache::{DbTable, SqliteDbError, UpdateHook};

/// Lightweight handle to a collection of entities that reloads all data from the database.
///
/// `StatelessCollection` is a simple collection type that re-queries all items from the database
/// whenever load_data() is called. It tracks which tables are relevant for change notifications,
/// and will indicate that updates are relevant when any of the tracked tables change.
///
/// This is called "stateless" because it doesn't track individual items in memory or apply
/// incremental updates. For better performance with large collections, use `ManagedCollection`
/// (to be implemented later).
///
/// # Type Parameter
/// - `T`: The type of items in the collection (typically `FullEntity<SomeType>`)
///
/// # Example Usage
/// ```rust,ignore
/// let collection = StatelessCollection::new(
///     vec![DbTable::PostsEditContext, DbTable::TermRelationships],
///     Box::new(move || {
///         // Load all posts from database
///         cache.execute(|conn| repo.select_all(conn, &site))
///     }),
/// );
///
/// // Later, when an update notification arrives:
/// if collection.is_relevant_update(&update_hook) {
///     let items = collection.load_data()?;
///     // ... notify observers with new data
/// }
/// ```
pub struct StatelessCollection<T> {
    /// Tables to monitor for changes
    relevant_tables: Vec<DbTable>,

    /// Closure that loads all collection data from the database
    load_data: Box<dyn Fn() -> Result<Vec<T>, SqliteDbError> + Send + Sync>,
}

impl<T> StatelessCollection<T> {
    /// Create a new stateless collection handle
    ///
    /// # Parameters
    /// - `relevant_tables`: List of tables to monitor for changes. When any of these tables
    ///   has an insert/update/delete, `is_relevant_update()` will return true.
    /// - `load_data`: Closure that loads all items from the database. This will be called
    ///   each time `load_data()` is invoked.
    ///
    /// The load_data closure is provided by the service layer and encapsulates the logic
    /// for reading collection data from the cache/DB, including which repository to use,
    /// which context, and any filtering logic.
    pub fn new(
        relevant_tables: Vec<DbTable>,
        load_data: Box<dyn Fn() -> Result<Vec<T>, SqliteDbError> + Send + Sync>,
    ) -> Self {
        Self {
            relevant_tables,
            load_data,
        }
    }

    /// Load all items in the collection from the database
    ///
    /// This is an expensive operation that reads from the database each time.
    /// It returns all items currently stored in the database that match the
    /// collection's criteria (site, context, etc.).
    ///
    /// Returns:
    /// - `Ok(Vec<T>)` - All items in the collection (may be empty)
    /// - `Err(SqliteDbError)` if a database error occurred
    pub fn load_data(&self) -> Result<Vec<T>, SqliteDbError> {
        (self.load_data)()
    }

    /// Check if a database update is relevant to this collection
    ///
    /// Returns true if the updated table is one of the tables this collection monitors.
    /// This allows platform-specific observable wrappers to determine whether they should
    /// notify observers about a database change.
    ///
    /// # Example
    /// A posts collection might monitor both `PostsEditContext` and `TermRelationships`
    /// tables, so any change to either table would be considered relevant.
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        self.relevant_tables.contains(&hook.table)
    }
}

// Note: PartialEq, Eq, and Clone are not implemented because the load_data closure
// cannot be cloned or compared. Collections are identified by their criteria conceptually,
// but Rust can't derive these traits with the closure field.

#[cfg(test)]
mod tests {
    use super::*;
    use wp_mobile_cache::HookAction;

    #[test]
    fn test_load_data_calls_closure() {
        let test_data = vec![1, 2, 3];
        let test_data_clone = test_data.clone();

        let collection = StatelessCollection::new(
            vec![DbTable::PostsEditContext],
            Box::new(move || Ok(test_data_clone.clone())),
        );

        let result = collection.load_data().expect("Should succeed");
        assert_eq!(result, test_data);
    }

    #[test]
    fn test_is_relevant_update_single_table() {
        let collection = StatelessCollection::<i32>::new(
            vec![DbTable::PostsEditContext],
            Box::new(|| Ok(vec![])),
        );

        let matching_hook = UpdateHook {
            action: HookAction::Update,
            db_name: "main".to_string(),
            table: DbTable::PostsEditContext,
            row_id: 42,
        };

        assert!(
            collection.is_relevant_update(&matching_hook),
            "Should match updates to monitored table"
        );

        let non_matching_hook = UpdateHook {
            action: HookAction::Insert,
            db_name: "main".to_string(),
            table: DbTable::PostsViewContext,
            row_id: 42,
        };

        assert!(
            !collection.is_relevant_update(&non_matching_hook),
            "Should not match updates to non-monitored table"
        );
    }

    #[test]
    fn test_is_relevant_update_multiple_tables() {
        let collection = StatelessCollection::<i32>::new(
            vec![DbTable::PostsEditContext, DbTable::TermRelationships],
            Box::new(|| Ok(vec![])),
        );

        let posts_hook = UpdateHook {
            action: HookAction::Update,
            db_name: "main".to_string(),
            table: DbTable::PostsEditContext,
            row_id: 1,
        };

        let term_hook = UpdateHook {
            action: HookAction::Delete,
            db_name: "main".to_string(),
            table: DbTable::TermRelationships,
            row_id: 2,
        };

        let other_hook = UpdateHook {
            action: HookAction::Insert,
            db_name: "main".to_string(),
            table: DbTable::SelfHostedSites,
            row_id: 3,
        };

        assert!(
            collection.is_relevant_update(&posts_hook),
            "Should match updates to first monitored table"
        );
        assert!(
            collection.is_relevant_update(&term_hook),
            "Should match updates to second monitored table"
        );
        assert!(
            !collection.is_relevant_update(&other_hook),
            "Should not match updates to non-monitored table"
        );
    }
}
