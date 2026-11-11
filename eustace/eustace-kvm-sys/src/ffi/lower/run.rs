//! Checked calls for opaque KVM run images and their active exits.

use core::mem::MaybeUninit;
use std::io::{Error, ErrorKind, Result};

use crate::ffi::binding::bridge;

pub mod io;

pub mod mmio;

/// Classification of the active KVM exit.
pub type Kind = bridge::nekor_kvm_run_exit_kind_t;

/// Original KVM exit reason retained for unsupported exits.
pub type Reason = bridge::nekor_kvm_exit_reason_t;

/// Classification and original KVM reason code for an exit.
#[derive(Debug)]
pub struct Outcome {
    /// Classification of the active exit.
    pub kind: Kind,

    /// Original KVM exit reason.
    pub reason: Reason,
}

/// Validate the fixed KVM run-image extent and its required alignment.
#[inline]
pub fn validate(mapping: &[u8]) -> Result<()> {
    let run_size = mapping.len().try_into().or(Err(Error::from(ErrorKind::InvalidInput)))?;

    // SAFETY:
    //
    // * The slice is live for the call. C checks its pointer, extent, and alignment.
    let target_status = unsafe { bridge::nekor_kvm_run_image_validate(mapping.as_ptr().cast(), run_size) };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => Ok(()),
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

/// Classify the active run exit and return its kind and original KVM reason.
#[inline]
pub fn exit(mapping: &[u8]) -> Result<Outcome> {
    let run_size = mapping.len().try_into().or(Err(Error::from(ErrorKind::InvalidInput)))?;

    let mut exit_kind = MaybeUninit::<Kind>::uninit();
    let mut exit_reason = MaybeUninit::<Reason>::uninit();

    // SAFETY:
    //
    // * The slice and both output locations remain valid for the bridge call.
    let target_status = unsafe {
        bridge::nekor_kvm_run_exit_classify(
            mapping.as_ptr().cast(),
            run_size,
            exit_kind.as_mut_ptr(),
            exit_reason.as_mut_ptr(),
        )
    };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => {
            // SAFETY:
            //
            // * The C function initializes both outputs before returning success.
            let exit_kind = unsafe { exit_kind.assume_init() };

            // SAFETY:
            //
            // * The C function initializes both outputs before returning success.
            let exit_reason = unsafe { exit_reason.assume_init() };

            Ok(Outcome {
                kind: exit_kind,
                reason: exit_reason,
            })
        },
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}
