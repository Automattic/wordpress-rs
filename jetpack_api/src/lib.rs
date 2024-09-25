#![allow(dead_code, unused_variables)]

pub use jetpack_client::{JetpackClient, JetpackRequestBuilder};

mod jetpack_client; // re-exported relevant types

pub mod jetpack_connection;
pub mod request;

uniffi::setup_scaffolding!();
