#![allow(dead_code, unused_variables)]

pub use api_error::{JetpackApiError, JetpackRequestExecutionError};
pub use jetpack_client::{JetpackClient, JetpackRequestBuilder};

mod api_error; // re-exported relevant types
mod jetpack_client; // re-exported relevant types

pub mod jetpack_connection;
pub mod request;

uniffi::setup_scaffolding!();
