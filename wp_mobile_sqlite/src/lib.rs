//! SQLite plumbing shared by the on-device databases.
//!
//! Opening and configuring a connection, guarding it behind a mutex, and
//! running a database's migrations. Nothing here knows about any table.

mod connection;
mod functions;
mod migrations;

pub use connection::{ConnectionError, DbConnection};
pub use migrations::MigrationManager;
