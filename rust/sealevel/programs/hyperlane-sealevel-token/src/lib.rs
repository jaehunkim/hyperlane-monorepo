//! Hyperlane Token program for synthetic tokens.

#![deny(warnings)]
#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod plugin;
pub mod processor;

pub use spl_associated_token_account;
pub use spl_noop;
pub use spl_token;
pub use spl_token_2022;

// FIXME Read these in at compile time? And don't use harcoded test keys.
solana_program::declare_id!("3MzUPjP5LEkiHH82nEAe28Xtz9ztuMqWc8UmuKxrpVQH");