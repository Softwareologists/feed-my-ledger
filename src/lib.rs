//! Rusty Ledger
//!
//! This crate provides an append-only immutable database that can interact with
//! cloud-based spreadsheet services.

pub mod cloud_adapters;
pub mod core;
pub mod import;
pub mod script;
