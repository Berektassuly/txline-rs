//! TxL purchase quote scaffolding.
//!
//! Paid flows request a quote from `/api/guest/purchase/quote` and receive a
//! partially signed Solana transaction. Future code must inspect the transaction
//! before signing and must not log wallet secrets or authorization headers.
