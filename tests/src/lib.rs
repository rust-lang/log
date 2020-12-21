//! This crate is intentionally left empty.
//! 
//! We have an empty library depending on `log` here so we can run integration tests
//! on older compilers without depending on the unstable `no-dev-deps` flag.

#![allow(dead_code)]

#[cfg(test)]
#[macro_use]
extern crate log;

#[cfg(test)]
#[path = "../filters.rs"]
mod filters;

#[cfg(test)]
#[path = "../macros.rs"]
mod macros;
