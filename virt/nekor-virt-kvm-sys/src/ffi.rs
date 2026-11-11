//! Foreign-function bindings and checked calls for the Nekor KVM interface.
//!
//! [`binding`] contains Linux KVM and Nekor ABI declarations. [`lower`] provides checked operations
//! for device descriptors, memory mappings, and virtual processors.

pub mod binding;

pub mod lower;

#[doc(hidden)]
pub mod layout;
