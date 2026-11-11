//! Percolator Agent Registry: an open, clean-room pairing registry that links a
//! Metaplex Core agent asset to any Solana token or market (SPL, Token-2022,
//! pump.fun curve, PumpSwap, Raydium, Meteora, Genesis, Percolator markets,
//! Phoenix or other perps).
//!
//! Written from scratch from public account layouts. It does not link or copy any
//! Metaplex crate or source; the Core asset bytes are parsed directly.
//!
//! Accounts
//!   RegistryConfig  PDA ["config"]                808 B
//!   PairingV1       PDA ["pairing", asset, target] 208 B
//!   PrimaryPointer  PDA ["primary", asset]         48 B
//!
//! Authority for every per-agent instruction is EITHER the asset owner signing
//! directly, OR the Core asset-signer PDA (["mpl-core-execute", asset] under
//! Core), which can only sign when Core `Execute` CPIs on the owner's behalf.
#![no_std]

pub mod state;
pub mod error;
pub mod processor;

pub use processor::process_instruction;

#[cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]
mod entry {
    use pinocchio::{no_allocator, nostd_panic_handler, program_entrypoint};
    program_entrypoint!(crate::process_instruction);
    no_allocator!();
    nostd_panic_handler!();
}
