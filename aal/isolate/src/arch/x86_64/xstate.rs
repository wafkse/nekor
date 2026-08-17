//! Extended Processor State management.

use nekor_aal_feature::prelude::Feature;

/// A marker trait to describe an architectural extended state.
///
/// # Safety
///
/// The provided [`Feature`] must correspond to the Xsave-managed state
/// for the indice specified.
pub unsafe trait XState {
    /// The feature associated to this extended state.
    type Feature: Feature;

    /// The feature bitmap indice for this extended state.
    const INDICE: u32;
}

// TODO: Pick out the states from XCR0 and define them here.
//
// Maybe make wrapper types to make them readable from an Xsave area.
