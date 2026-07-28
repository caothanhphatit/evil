//! Evidence-backed combat arithmetic shared by formula analyzers and the runtime.
//!
//! Modules in this namespace stay disconnected from live outcomes until their
//! complete caller contracts have native evidence and golden vectors.

#![allow(dead_code)]

pub(crate) mod arithmetic;
pub(crate) mod critical;
pub(crate) mod hit_resolution;
pub(crate) mod hunter_incoming;
pub(crate) mod monster_incoming;
pub(crate) mod outgoing;
pub(crate) mod runtime;
pub(crate) mod skill;
pub(crate) mod status_damage;
