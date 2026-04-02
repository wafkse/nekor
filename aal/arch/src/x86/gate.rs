//! Interrupt Gates.

/// The nibble-sized type field of a *Gate Descriptor*.
///
/// # Gate Types
///
/// In *Real Mode*, there is no such thing as a *Gate Descriptor* and thus there
/// are no *Gate Type*s.
///
/// ## Interrupt Gate
///
/// An interrupt gate defines the *Interrupt Service Routine* as non-preemptable
/// at the interrupt-level.
///
/// In other words, the processor takes care of configuring `IF` in the `FLAGS`
/// register when serving an *Interrupt Service Routine* of this type.
///
/// ## Trap Gate
///
/// Identical to an *Interrupt Gate*, but the ISR can be preempted by another
/// interrupt.
///
/// ## Task Gate
///
/// When the associated interrupt vector is dispatched, the processor will
/// switch to the target *Task State Segment* pointed by the *Segment Selector*
/// in the *Gate Descriptor*.
///
/// The *Task Link* in the newly-switched *TSS* will be set to point to the
/// *TSS* that raised the interrupt.
///
/// **Note**: Hardware multitasking is not used in Nekor, and the
/// [`GateType::Task`] variant simply exists for completeness. It is recommended
/// to amalgamate its behavior with non-exhaustivess handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum GateType {
    /// A task gate intended for hardware-driven multitasking. Only supported in
    /// *Protected Mode*.
    ///
    /// The raw *Gate Type* for this variant is `0x5` (nibble-sized).
    Task,

    /// A 16-bit interrupt gate type.
    ///
    /// This is not accessible from regular kernel code.
    ///
    /// # Disallowed
    ///
    /// This gate type is disallowed, as the kernel is *Long Mode-*only.
    Bits16(GateType16),

    /// A 32-bit interrupt gate type.
    ///
    /// # Disallowed
    ///
    /// This gate type is disallowed, as the kernel is *Long Mode-*only.
    Bits32(GateType32),
}

/// An interrupt [`GateType`] intended for 16-bit code execution.
///
/// This *Gate Type* is only supported in a *Protected Mode* context, as a *Real
/// Mode* *Interrupt Descriptor Table* does not have per-entry *Gate
/// Descriptors*.
///
/// # Effects on Interrupt Service Routine Dispatch
///
/// For a 16-bit *Gate Type*, the processor will truncate the *Offset* field in
/// the *Gate Descriptor* to `16`-bits.
///
/// Furthermore, the pushed interrupt data is also made 16-bits long.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
#[non_exhaustive]
pub enum GateType16 {
    /// A *Protected Mode* 16-bit interrupt mode.
    Interrupt = 0x06,

    /// A *Protected Mode* 16-bit interrupt trap mode.
    Trap = 0x07,
}

/// An interrupt [`GateType`] intended for *Protected Mode* code execution.
///
/// This *Gate Type* is only supported in a *Protected Mode* context.
///
/// # Effects on Interrupt Service Routine Dispatch
///
/// The only effects depend on whether it is an *Interrupt Gate* or *Trap Gate*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
#[non_exhaustive]
pub enum GateType32 {
    /// A *Protected Mode* 32-bit interrupt mode.
    Interrupt = 0x0E,

    /// A *Protected Mode* 32-bit interrupt trap mode.
    Trap = 0x0F,
}
