#![no_std]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    rustdoc::all
)]
//! The boot protocol and base structure information to be implemented by
//! bootloader crates.
//!
//! The Nekor kernel expects a very precise environment to be loaded in. This is
//! meant to serve as a way to express such requirements even across distinct
//! kernel and bootloader version pairs.
//!
//! # Nekor Boot Protocol Specification
//!
//! The Nekor Boot Protocol defines a decentralized Type-Length-Value interface
//! designed to safely facilitate the environment handoff between the bootloader
//! and the operating system. Traditional boot protocols utilize monolithic
//! header structures or centralized arrays of requests. Such implementations
//! suffer from structural bloat due to alignment padding, require highly
//! centralized maintenance, or force the kernel binary to carry configuration
//! data for unconsumed features. The Nekor Boot Protocol shifts the burden of
//! structural layout directly to the compiler or linker.
//!
//! ## Initialization Handshake
//!
//! The protocol establishes a fail-fast handshake prior to memory mapping or
//! page table initialization. The kernel embeds its overarching feature
//! requirements directly into the program dynamic interpreter string of the
//! executable, placed within the `PT_INTERP` segment. The bootloader reads this
//! interpreter string upon parsing the executable. If the bootloader encounters
//! an unsupported protocol version, it halts execution immediately. This
//! guarantees that complex memory parsing remains gated behind a strict
//! compatibility check.
//!
//! ## Linker Layout Strategy
//!
//! Kernel subsystems autonomously declare bootloader requirements by
//! instantiating standalone static blocks. To unify these isolated requests
//! without wasting memory on standard struct alignment padding, the protocol
//! utilizes lexicographical sorting at the linker level. Every bootloader
//! request is placed into a custom executable section prefixed with
//! `.nekor.boot.`, followed by a zero-padded numeric string.
//!
//! The kernel enforces a primary boot record, referred to as the anchor, which
//! is assigned to `.nekor.boot.000record`. During compilation, the linker
//! aligns the base of the combined `.nekor.boot` segment to a page boundary and
//! sorts all contained sections by name. This guarantees the primary boot
//! record sits at the exact page-aligned start of the block, followed
//! immediately by all subsequent subsystem requests packed contiguously with
//! zero alignment holes.
//!
//! ### Section Organization Map
//!
//! | Virtual Offset | Section Designation         | Exported Symbol             | Alignment | Content Description |
//! |----------------|-----------------------------|-----------------------------|-----------|---------------------|
//! | Base + 0x0000  | `.nekor.boot.000record`     | `<nekor::boot::record>`     | 4096      | The primary boot record anchor. |
//! | Base + 0x0010  | `.nekor.boot.010features`   | `<nekor::boot::features>`   | 8         | The feature request block. |
//! | Base + 0x0030  | `.nekor.boot.020framebuf`   | `<nekor::boot::framebuf>`   | 8         | The framebuffer request block. |
//! | Base + ...     | `.nekor.boot.[N][name]`     | `<nekor::boot::[name]>`     | 8         | Subsequent contiguous requests. |
//!
//! ## Type Length Value Architecture
//!
//! The bootloader does not rely on a hardcoded sequence to fulfill requests.
//! The physical layout of the memory block shifts depending on the compiled
//! modules. To navigate this dynamic payload, the protocol employs a
//! Type-Length-Value architecture. Every packed request begins with a
//! standardized sixteen-byte header. The type is represented by a unique
//! identifier designated as a Slug, which is a deterministic FNV-1a hash of the
//! request signature. The length is explicitly defined within the header,
//! declaring the total byte footprint of the entire request or response block.
//!
//! ### Block Memory Layout
//!
//! | Byte Offset | Size (Bytes) | Field Designation | Data Type      | Functional Description |
//! |-------------|--------------|-------------------|----------------|------------------------|
//! | 0x00        | 8            | `slug_id`         | `Le<u64>`      | Deterministic identifier for the request type. |
//! | 0x08        | 8            | `block_length`    | `Le<u64>`      | Total size of the header plus the storage union. |
//! | 0x10        | Varies       | `storage_union`   | `UnsafeCell`   | The overlapping request or response data payload. |
//!
//! ## Forward Compatibility
//!
//! The bootloader sweeps the contiguous memory block starting from the anchor
//! by reading headers in sequence. Upon encountering a recognized Slug, it
//! executes the specific hardware provisioning routine. If the bootloader
//! encounters an unknown Slug originating from a newer kernel specification, it
//! utilizes the block length field to skip over the unrecognized data safely.
//! This mechanism renders the protocol forward-compatible against unknown
//! payloads.
//!
//! ## Memory Provisioning
//!
//! The core payload utilizes an overlapping storage union to maximize memory
//! efficiency. Once the bootloader successfully provisions a requested
//! resource, it writes the resulting metadata directly back into the exact
//! memory footprint of the original request. This overwrites the request fields
//! with the populated response structure prior to transferring control to the
//! kernel entry point.
//!
//! ## Dead Code Elimination
//!
//! The protocol enforces strict dead code elimination. Static requests within
//! the kernel are not marked with compiler preservation attributes. If a kernel
//! module defines a request but fails to invoke the code required to read the
//! response, the compiler aggressively strips the static structure from the
//! final binary before linker evaluation. This guarantees the kernel never
//! requests resources it does not actively consume.
//!
//! ## Symbol Namespace Isolation
//!
//! The protocol exports its anchors using synthetic symbol names to guarantee
//! safety between the build toolchain or the runtime environment. By injecting
//! characters such as angle brackets directly into the symbol table via inline
//! assembly, the protocol ensures that standard runtime code cannot
//! accidentally reference or mutate the bootloader metadata. The structures
//! remain entirely isolated from the standard memory namespace of the operating
//! system.

pub mod protocol;

pub mod storage;

// NOTE: The bootloader feature-request string.
//
// In a non-usermode kernel build, this is stated as the required program
// dynamic interpreter.
#[cfg(feature = "kernel")]
core::arch::global_asm!(
    concat!(
        ".pushsection",
        ' ',
        env!("BOOTLOADER_FEATURE_SECTION"),
        ',',
        // NOTE: Only `SHF_ALLOC` is required for the *Bootloader Request Section*.
        '"',
        'a',
        '"',
        ',',
        "@progbits"
    ),
    concat!('"', env!("BOOTLOADER_FEATURE_SYMBOL"), '"', ':'),
    concat!(".global", ' ', '"', env!("BOOTLOADER_FEATURE_SYMBOL"), '"'),
    concat!(
        ".type",
        ' ',
        '"',
        env!("BOOTLOADER_FEATURE_SYMBOL"),
        '"',
        ',',
        ' ',
        "@object"
    ),
    concat!(".asciz", ' ', '"', env!("BOOTLOADER_FEATURE_LIST"), '"'),
    concat!(
        ".size",
        ' ',
        '"',
        env!("BOOTLOADER_FEATURE_SYMBOL"),
        '"',
        ',',
        ' ',
        '.',
        '-',
        ' ',
        '"',
        env!("BOOTLOADER_FEATURE_SYMBOL"),
        '"',
    ),
    ".popsection"
);
