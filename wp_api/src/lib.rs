#![allow(dead_code, unused_variables)]

pub use pages::*;
pub use posts::*;

pub mod pages;
pub mod posts;

uniffi::include_scaffolding!("wp_api");
