//#![deny(missing_docs)]
//#![forbid(unsafe_code)]
#![deny(warnings)]
#![allow(deprecated)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

//! An ERC20-like Token program for the Solana blockchain
#[macro_use]
mod debug;
#[macro_use]
mod error;
pub mod macrorules;
pub mod config;
pub mod entrypoint;
/// hamt impl
pub mod hamt;
pub mod solana_backend;
pub mod account_data;
pub mod account_storage;
pub mod solidity_account;
mod storage_account;
pub mod instruction;
mod transaction;
/// Todo: document
pub mod executor;
/// Todo: document
pub mod executor_state;
pub mod utils;
pub mod operator;
pub mod payment;
pub mod token;
pub mod system;
pub mod precompile_contracts;

// Export current solana-sdk types for downstream users who may also be building with a different
// solana-sdk version
pub use solana_program;

//solana_sdk::declare_id!("EVM1111111111111111111111111111111111111111");
#[cfg(feature = "tracing")]
pub mod tracing;

#[cfg(feature = "tracing")]
macro_rules! event {
    ($x:expr) => {
        use crate::tracing::Event::*;
        crate::tracing::with(|listener| listener.event($x));
    };
}

#[cfg(feature = "tracing")]
macro_rules! emit_exit {
    ($reason:expr) => {{
        let reason = $reason;
        event!(Exit {
            reason: &reason.into(),
            return_value: &Vec::new(),
        });
        reason
    }};
    ($return_value:expr, $reason:expr) => {{
        let reason = $reason;
        let return_value = $return_value;
        event!(Exit {
            reason: &reason,
            return_value: &return_value,
        });
        (return_value, reason)
    }};
}

#[cfg(not(feature = "tracing"))]
macro_rules! emit_exit {
    ($reason:expr) => {
        $reason
    };
    ($return_value:expr, $reason:expr) => {
        ($return_value, $reason)
    };
}

#[cfg(not(feature = "tracing"))]
macro_rules! event {
    ($x:expr) => {};
}

pub(crate) use event;
pub(crate) use emit_exit;
