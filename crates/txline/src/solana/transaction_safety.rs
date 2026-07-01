//! Transaction safety scaffolding.
//!
//! Purchase quote transactions must be inspected before signing. Future checks
//! should verify the fee payer, expected backend/admin signature, allowed
//! programs, signer roles, decoded instruction name, requested amount, and that
//! no extra oracle instructions are present.
