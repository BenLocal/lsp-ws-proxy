//! Nonstandard LSP features.
mod relative_uri;
mod sqls;

pub use relative_uri::remap_relative_uri;
pub use sqls::create_database_on_init;
