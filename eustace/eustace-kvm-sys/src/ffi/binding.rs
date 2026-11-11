//! Generated C declarations grouped by the ABI they represent.

pub mod kvm {
    //! Declarations from the selected Linux KVM UAPI.

    #[cfg(feature = "lts")]
    pub use crate::generated::lts;
    #[cfg(feature = "stable")]
    pub use crate::generated::stable;

    cfg_select! {
        all(feature = "stable", feature = "lts") => {
            pub use crate::generated::stable as preferred;
        },
        feature = "stable" => {
            pub use crate::generated::stable as preferred;
        },
        feature = "lts" => {
            pub use crate::generated::lts as preferred;
        },
    }
}

/// Declarations for the Nekor-owned C bridge ABI shared by the supported kernel pins.
pub use crate::generated::bridge;
