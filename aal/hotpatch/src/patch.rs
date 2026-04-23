//! A delegation-based, type-safe, runtime code patching system.
//!
//! In this model, a [`Delegator`] type employs a [`Pod`] type to choose a
//! [`Delegated`] implementation.
//!
//! [`Delegator`]: delegate::Delegator
//! [`Delegated`]: delegate::Delegated
//! [`Pod`]: pod::Pod

pub mod delegate;

pub mod pod;

pub mod choose;

pub mod patch;
