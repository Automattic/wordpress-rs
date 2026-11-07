//! SQLite plumbing shared by the on-device databases.
//!
//! Opening and configuring a connection, guarding it behind a mutex, running a
//! database's migrations, and reading rows by column enum. Nothing here knows
//! about any table.

mod connection;
mod functions;
mod migrations;
mod row_ext;

pub use connection::{ConnectionError, DbConnection};
pub use migrations::MigrationManager;
pub use row_ext::{ColumnIndex, RowExt};
