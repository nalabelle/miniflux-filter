//! Filter Core - Core filtering functionality for Miniflux RSS reader

pub mod api;
pub mod config;
pub mod filter;
pub mod rules;

pub type Result<T> = anyhow::Result<T>;
