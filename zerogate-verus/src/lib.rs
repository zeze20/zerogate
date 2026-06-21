// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Pure formal verification model for ZeroGate.
//!
//! This crate contains no syscalls, raw pointers, global mutable state,
//! or OS APIs. All functions are pure, deterministic, and side-effect-free.

pub mod model;
