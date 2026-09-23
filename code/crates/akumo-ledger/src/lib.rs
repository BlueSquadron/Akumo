//! # akumo-ledger
//!
//! The Persistence Port adapter (ADR-0014): an append-only, hash-chained event store, engagement
//! isolated (DSR-2) and crash-safe (NFR-REL1). It is the single source of truth from which every
//! projection is folded (see [`akumo_core::projection`]).
//!
//! ## Substrate
//!
//! v1 uses a **JSON-Lines file per engagement** (`<root>/<engagement>.jsonl`): append one event
//! per line, `fsync` on append. This is deliberately simple and *inspectable* (DSR-1) — an operator
//! can read the ledger with `cat`/`jq`. The Increment 7 packaging pass may swap the substrate for an
//! embedded KV store (IMPL-CHOICE `redb`) behind this same port without touching the core.

#![forbid(unsafe_code)]

mod file_store;

pub use akumo_domain as domain;
pub use file_store::FileEventStore;
