//! Tightly-packed raw versions of interrupt structures.

use core::{arch, marker, mem, num::NonZeroU32};

use nekor_aal_agnostic::partitioned::Partitioned;
use nekor_aal_arch::{x86::privilege::PrivilegeLevel, x86_64::gate::GateType64};
use nekor_bitwise::prelude::{Bit, Counterpart, Field, State};
use zerocopy::{Immutable, IntoBytes};

/// Private sealing implementation for [`Vector`].
mod private {
    /// A seal supertrait to the [`Vector`] trait.
    ///
    /// [`Vector`]: super::Vector
    pub trait Sealed {}
}

/// A raw index into an *Interrupt Stack Table*.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(u8)]
pub enum RawIst {
    /// The *1st* *Interrupt Stack Table* field.
    Ist1 = 0b001,

    /// The *2nd* *Interrupt Stack Table* field.
    Ist2 = 0b010,

    /// The *3rd* *Interrupt Stack Table* field.
    Ist3 = 0b011,

    /// The `4th` *Interrupt Stack Table* field.
    Ist4 = 0b100,

    /// The `5th` *Interrupt Stack Table* field.
    Ist5 = 0b101,

    /// The *6th* *Interrupt Stack Table* field.
    Ist6 = 0b110,

    /// The `7th` *Interrupt Stack Table* field.
    Ist7 = 0b111,
}

/// An enumeration that determines the presence (and, ultimately its validity)
/// of a *Gate Descriptor*.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(u8)]
pub enum Present {
    /// No, the specified [`RawGateDescriptor`] is not present.
    ///
    /// An *Interrupt Service Routine* dispatch to the gate will result in a
    /// *General Protection* fault.
    No = 0b0000_0000,

    /// Yes, the specified [`RawGateDescriptor`] is present.
    Yes = 0b0000_0001,
}

/// The bitwise field of the *Gate Type* field of this [`RawTypeAttributes`].
///
/// This is a 4-bit field that determines the type of gate (interrupt or trap).
pub type TypeAttributesGateType<'a> = Field<'a, 0, 3, u8>;

/// The bitwise field of the *Privilege Level* field of this
/// [`RawTypeAttributes`].
///
/// This is a 2-bit field that determines the required privilege level.
pub type TypeAttributesPrivilegeLevel<'a> = Field<'a, 5, 6, u8>;

/// The bit of the *Present* field of this [`RawTypeAttributes`].
///
/// This determines whether the gate descriptor is present and valid.
pub type TypeAttributesPresent<'a> = Bit<'a, u8, 7>;

/// The mutable bitwise field of the *Gate Type* field of this
/// [`RawTypeAttributes`].
pub type TypeAttributesGateTypeMut<'a> = <TypeAttributesGateType<'a> as Counterpart>::Mut;

/// The mutable bitwise field of the *Privilege Level* field of this
/// [`RawTypeAttributes`].
pub type TypeAttributesPrivilegeLevelMut<'a> = <TypeAttributesPrivilegeLevel<'a> as Counterpart>::Mut;

/// The mutable bit of the *Present* field of this [`RawTypeAttributes`].
pub type TypeAttributesPresentMut<'a> = <TypeAttributesPresent<'a> as Counterpart>::Mut;

/// A **raw** 64-bit *Gate Descriptor* type attribute byte.
///
/// # Layout
///
/// | Bits | Field           | Size (bits) | Description                    |
/// |------|-----------------|-------------|--------------------------------|
/// | 3-0  | Gate Type       | 4           | Type of gate (interrupt/trap)  |
/// | 4    | Reserved        | 1           | Must be 0                      |
/// | 6-5  | Privilege Level | 2           | Descriptor Privilege Level     |
/// | 7    | Present         | 1           | Present bit                    |
///
/// The bit layout exactly matches the CPU representation.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, IntoBytes, Immutable)]
#[repr(transparent)]
// NOTE(invariant): Every byte is preserved verbatim; semantic packing is only a convenience.
pub struct RawTypeAttributes(u8);

impl RawTypeAttributes {
    /// Constructs a raw type-attribute byte.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u8) -> Self {
        Self(target_value)
    }

    /// Returns the raw type-attribute byte.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u8 {
        let Self(target_value) = self;

        target_value
    }

    /// Construct a zeroed [`RawTypeAttributes`].
    #[inline]
    #[must_use]
    pub const fn zeroed() -> Self {
        Self(u8::MIN)
    }

    /// Access the `Gate Type` field (bits 0–3) of this `RawTypeAttributes`.
    #[inline]
    #[must_use]
    pub const fn gate_type(&self) -> TypeAttributesGateType<'_> {
        let &Self(ref target_value) = self;

        TypeAttributesGateType::wrap(target_value)
    }

    /// Mutably access the `Gate Type` field (bits 0–3) of this
    /// `RawTypeAttributes`.
    #[inline]
    pub const fn gate_type_mut(&mut self) -> TypeAttributesGateTypeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        TypeAttributesGateTypeMut::wrap(target_value)
    }

    /// Access the `Privilege Level` field (bits 5–6) of this
    /// `RawTypeAttributes`.
    #[inline]
    #[must_use]
    pub const fn privilege_level(&self) -> TypeAttributesPrivilegeLevel<'_> {
        let &Self(ref target_value) = self;

        TypeAttributesPrivilegeLevel::wrap(target_value)
    }

    /// Mutably access the `Privilege Level` field (bits 5–6) of this
    /// `RawTypeAttributes`.
    #[inline]
    pub const fn privilege_level_mut(&mut self) -> TypeAttributesPrivilegeLevelMut<'_> {
        let &mut Self(ref mut target_value) = self;

        TypeAttributesPrivilegeLevelMut::wrap(target_value)
    }

    /// Access the `Present` bit (bit 7) of this `RawTypeAttributes`.
    #[inline]
    #[must_use]
    pub const fn present(&self) -> TypeAttributesPresent<'_> {
        let &Self(ref target_value) = self;

        TypeAttributesPresent::wrap(target_value)
    }

    /// Mutably access the `Present` bit (bit 7) of this `RawTypeAttributes`.
    #[inline]
    pub const fn present_mut(&mut self) -> TypeAttributesPresentMut<'_> {
        let &mut Self(ref mut target_value) = self;

        TypeAttributesPresentMut::wrap(target_value)
    }

    /// Pack a 3-tuple ([`GateType64`], [`PrivilegeLevel`], [`Present`]) into a
    /// [`RawTypeAttributes`] structure.
    #[inline]
    #[must_use]
    pub const fn pack(target_tuple: (GateType64, PrivilegeLevel, Present)) -> Self {
        let (gate_type, privilege_level, is_present) = target_tuple;

        let mut target_attributes = Self::zeroed();

        target_attributes.gate_type_mut().const_merge(gate_type as u8);

        target_attributes
            .privilege_level_mut()
            .const_merge(privilege_level as u8);

        let target_state = if matches!(is_present, Present::Yes) {
            State::Set
        } else {
            State::Cleared
        };

        target_attributes.present_mut().const_set(target_state);

        target_attributes
    }
}

/// Low sixty-four bits of a long-mode gate descriptor image.
pub type GateDescriptorLow = Partitioned<0, 63, u128>;

/// High sixty-four bits of a long-mode gate descriptor image.
pub type GateDescriptorHigh = Partitioned<64, 127, u128>;

/// Low handler-offset field in a long-mode gate descriptor.
pub type GateOffsetLow<'value> = Field<'value, 0, 15, u64>;

/// Mutable counterpart to [`GateOffsetLow`].
pub type GateOffsetLowMut<'value> = <GateOffsetLow<'value> as Counterpart>::Mut;

/// Code-segment selector field in a long-mode gate descriptor.
pub type GateSegmentSelector<'value> = Field<'value, 16, 31, u64>;

/// Mutable counterpart to [`GateSegmentSelector`].
pub type GateSegmentSelectorMut<'value> = <GateSegmentSelector<'value> as Counterpart>::Mut;

/// Interrupt-stack-table field in a long-mode gate descriptor.
pub type GateIst<'value> = Field<'value, 32, 34, u64>;

/// Mutable counterpart to [`GateIst`].
pub type GateIstMut<'value> = <GateIst<'value> as Counterpart>::Mut;

/// Reserved low-word field spanning descriptor bits 35 through 39.
pub type GateReserved35_39<'value> = Field<'value, 35, 39, u64>;

/// Mutable counterpart to [`GateReserved35_39`].
pub type GateReserved35_39Mut<'value> = <GateReserved35_39<'value> as Counterpart>::Mut;

/// Type-attribute byte in a long-mode gate descriptor.
pub type GateTypeAttributes<'value> = Field<'value, 40, 47, u64>;

/// Mutable counterpart to [`GateTypeAttributes`].
pub type GateTypeAttributesMut<'value> = <GateTypeAttributes<'value> as Counterpart>::Mut;

/// Middle handler-offset field in a long-mode gate descriptor.
pub type GateOffsetMiddle<'value> = Field<'value, 48, 63, u64>;

/// Mutable counterpart to [`GateOffsetMiddle`].
pub type GateOffsetMiddleMut<'value> = <GateOffsetMiddle<'value> as Counterpart>::Mut;

/// High handler-offset field in a long-mode gate descriptor.
pub type GateOffsetHigh<'value> = Field<'value, 0, 31, u64>;

/// Mutable counterpart to [`GateOffsetHigh`].
pub type GateOffsetHighMut<'value> = <GateOffsetHigh<'value> as Counterpart>::Mut;

/// Reserved high-word field spanning descriptor bits 96 through 127.
pub type GateReserved96_127<'value> = Field<'value, 32, 63, u64>;

/// Mutable counterpart to [`GateReserved96_127`].
pub type GateReserved96_127Mut<'value> = <GateReserved96_127<'value> as Counterpart>::Mut;

/// A 128-bit raw long-mode *Gate Descriptor* image.
///
/// Reference: <https://wiki.osdev.org/Interrupt_Descriptor_Table#Structure_on_x86-64>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, IntoBytes, Immutable)]
#[repr(C, align(8))]
// NOTE(invariant): The two partitions form one exact sixteen-byte architectural
// image. No raw bit pattern is normalized or rejected by this type.
pub struct RawGateDescriptor(GateDescriptorLow, GateDescriptorHigh);

impl RawGateDescriptor {
    /// Constructs a raw gate-descriptor image.
    #[inline]
    #[must_use]
    pub const fn new(low: u64, high: u64) -> Self {
        Self(GateDescriptorLow::raw(low), GateDescriptorHigh::raw(high))
    }

    /// Returns the low descriptor word.
    #[inline]
    #[must_use]
    pub const fn low(&self) -> &GateDescriptorLow {
        let &Self(ref low, ..) = self;

        low
    }

    /// Returns mutable access to the low descriptor word.
    #[inline]
    pub const fn low_mut(&mut self) -> &mut GateDescriptorLow {
        let &mut Self(ref mut low, ..) = self;

        low
    }

    /// Returns the high descriptor word.
    #[inline]
    #[must_use]
    pub const fn high(&self) -> &GateDescriptorHigh {
        let &Self(_, ref high) = self;

        high
    }

    /// Returns mutable access to the high descriptor word.
    #[inline]
    pub const fn high_mut(&mut self) -> &mut GateDescriptorHigh {
        let &mut Self(_, ref mut high) = self;

        high
    }

    /// Returns the low handler-offset field.
    #[inline]
    #[must_use]
    pub const fn offset_low(&self) -> GateOffsetLow<'_> {
        let &Self(ref low, ..) = self;

        GateOffsetLow::wrap(low.value())
    }

    /// Returns mutable access to the low handler-offset field.
    #[inline]
    pub const fn offset_low_mut(&mut self) -> GateOffsetLowMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        GateOffsetLowMut::wrap(low.value_mut())
    }

    /// Returns the code-segment selector field.
    #[inline]
    #[must_use]
    pub const fn segment_selector(&self) -> GateSegmentSelector<'_> {
        let &Self(ref low, ..) = self;

        GateSegmentSelector::wrap(low.value())
    }

    /// Returns mutable access to the code-segment selector field.
    #[inline]
    pub const fn segment_selector_mut(&mut self) -> GateSegmentSelectorMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        GateSegmentSelectorMut::wrap(low.value_mut())
    }

    /// Returns the raw three-bit interrupt-stack-table field.
    #[inline]
    #[must_use]
    pub const fn interrupt_stack_table(&self) -> GateIst<'_> {
        let &Self(ref low, ..) = self;

        GateIst::wrap(low.value())
    }

    /// Returns mutable access to the raw interrupt-stack-table field.
    #[inline]
    pub const fn interrupt_stack_table_mut(&mut self) -> GateIstMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        GateIstMut::wrap(low.value_mut())
    }

    /// Returns reserved descriptor bits 35 through 39.
    #[inline]
    #[must_use]
    pub const fn reserved_35_39(&self) -> GateReserved35_39<'_> {
        let &Self(ref low, ..) = self;

        GateReserved35_39::wrap(low.value())
    }

    /// Returns mutable access to reserved descriptor bits 35 through 39.
    #[inline]
    pub const fn reserved_35_39_mut(&mut self) -> GateReserved35_39Mut<'_> {
        let &mut Self(ref mut low, ..) = self;

        GateReserved35_39Mut::wrap(low.value_mut())
    }

    /// Returns the type-attribute byte field.
    #[inline]
    #[must_use]
    pub const fn type_attributes(&self) -> GateTypeAttributes<'_> {
        let &Self(ref low, ..) = self;

        GateTypeAttributes::wrap(low.value())
    }

    /// Returns mutable access to the type-attribute byte field.
    #[inline]
    pub const fn type_attributes_mut(&mut self) -> GateTypeAttributesMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        GateTypeAttributesMut::wrap(low.value_mut())
    }

    /// Returns the middle handler-offset field.
    #[inline]
    #[must_use]
    pub const fn offset_middle(&self) -> GateOffsetMiddle<'_> {
        let &Self(ref low, ..) = self;

        GateOffsetMiddle::wrap(low.value())
    }

    /// Returns mutable access to the middle handler-offset field.
    #[inline]
    pub const fn offset_middle_mut(&mut self) -> GateOffsetMiddleMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        GateOffsetMiddleMut::wrap(low.value_mut())
    }

    /// Returns the high handler-offset field.
    #[inline]
    #[must_use]
    pub const fn offset_high(&self) -> GateOffsetHigh<'_> {
        let &Self(_, ref high) = self;

        GateOffsetHigh::wrap(high.value())
    }

    /// Returns mutable access to the high handler-offset field.
    #[inline]
    pub const fn offset_high_mut(&mut self) -> GateOffsetHighMut<'_> {
        let &mut Self(_, ref mut high) = self;

        GateOffsetHighMut::wrap(high.value_mut())
    }

    /// Returns reserved descriptor bits 96 through 127.
    #[inline]
    #[must_use]
    pub const fn reserved_96_127(&self) -> GateReserved96_127<'_> {
        let &Self(_, ref high) = self;

        GateReserved96_127::wrap(high.value())
    }

    /// Returns mutable access to reserved descriptor bits 96 through 127.
    #[inline]
    pub const fn reserved_96_127_mut(&mut self) -> GateReserved96_127Mut<'_> {
        let &mut Self(_, ref mut high) = self;

        GateReserved96_127Mut::wrap(high.value_mut())
    }
}

/// Assert that the this *Gate Descriptor* attribute is 16-bytes long and is
/// aligned to a 8-byte boundary.
const _: () = {
    assert!(
        mem::size_of::<RawGateDescriptor>() == mem::size_of::<u128>(),
        "raw gate descriptors must occupy 128 bits"
    );

    assert!(
        mem::align_of::<RawGateDescriptor>() == mem::align_of::<usize>(),
        "raw gate descriptors must be naturally aligned"
    );
};

#[cfg(test)]
mod representation_tests {
    use zerocopy::IntoBytes;

    use super::{RawGateDescriptor, RawTypeAttributes};

    #[test]
    fn raw_gate_preserves_complete_descriptor_image() {
        let low = 0x0123_4567_89ab_cdef_u64;
        let high = 0xfedc_ba98_7654_3210_u64;
        let gate = RawGateDescriptor::new(low, high);
        let mut expected = [0_u8; 16];

        expected[..8].copy_from_slice(&low.to_ne_bytes());
        expected[8..].copy_from_slice(&high.to_ne_bytes());

        assert_eq!(*gate.low().value(), low);
        assert_eq!(*gate.high().value(), high);
        assert_eq!(gate.as_bytes(), expected);
    }

    #[test]
    fn raw_gate_reserved_fields_are_not_normalized() {
        let gate = RawGateDescriptor::new(u64::MAX, u64::MAX);

        assert_eq!(gate.interrupt_stack_table().const_value(), 0b111);
        assert_eq!(gate.reserved_35_39().const_value(), 0b1_1111);
        assert_eq!(gate.type_attributes().const_value(), u8::MAX);
        assert_eq!(gate.reserved_96_127().const_value(), u32::MAX);
        assert_eq!(RawTypeAttributes::new(0b0001_0000).raw(), 0b0001_0000);
    }
}

/// A marker type to determine whether the `K`th *Interrupt Vector* is valid.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Vector<const K: u8> {}

/// A marker trait to indicate that the `N`th *Interrupt Vector* is valid and
/// available for use.
pub trait Available: private::Sealed {}

/// Implement the [`Available`] trait for a sequence of interrupt vectors.
macro_rules! vector {
    (
        [
            $($target_vector:literal),+ $(,)?
        ] for $target_ty:ident
    ) => {
        $(
            impl private::Sealed for $target_ty<$target_vector> {}

            impl Available for $target_ty<$target_vector> {}
        )+
    };
}

vector!(
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255] for Vector
);

/// A token to serve as proof for an *Interrupt Service Routine*.
#[derive(Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct Service(
    // NOTE(invariant): Do not make this `Sync`.
    marker::PhantomData<*mut Self>,
);

impl Service {
    /// Provide a [`Service`] token to an *Interrupt Service Routine*.
    ///
    /// # Safety
    ///
    /// - The stack must be either:
    ///     - Genuine to the standard stack layout and in an actual processor-invoked interrupt.
    ///     - In the genuine stack format, within the guarantees of non-reentrancy[^1] for the
    ///       specific interrupt vector and gate type.
    ///
    /// [^1]: Does not apply to *non-maskable interrupts* when in a *pseudo-interrupt* (forged or otherwise simulated) context.
    ///
    /// ## Forgery
    ///
    /// Forgery of a [`Service`] token is __only__ acceptable in the case of
    /// deferred *Interrupt Service Routine*.
    #[inline]
    #[must_use]
    pub const unsafe fn provide() -> Self {
        Self(marker::PhantomData)
    }
}

/// A 32-bit *Error Code* that is obtained through interrupt vectors that signal
/// its existence.
#[derive(Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
#[repr(transparent)]
pub struct ErrorCode(NonZeroU32);

/// The *environmental constraint* imposed to a *Interrupt Service Routine*.
///
/// That is, whether the *Interrupt Service Routine* expects a *Trap* or
/// *Interrupt* gate type.
pub trait Environmental: private::Sealed {}

/// A marker type for a *Trap Gate*-based *Interrupt Service Routine*.
#[derive(Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub enum Trapped {}

/// A marker type for a *Interrupt Gate*-based *Interrupt Service Routine*.
#[derive(Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub enum Interrupted {}

impl private::Sealed for Trapped {}

impl private::Sealed for Interrupted {}

impl Environmental for Trapped {}

impl Environmental for Interrupted {}

/// A trait indicating the [`Routine::raw`] implementation for a trait whose
/// supertrait is [`Routine`].
pub trait Forward: private::Sealed {
    #[doc(hidden/* reason = "this function must be a last resort" */)]
    #[unsafe(naked)]
    unsafe extern "C" fn raw(target_service: &Service) -> ! {
        arch::naked_asm!("ud2", options(att_syntax))
    }
}

#[repr(transparent)]
pub struct Become<I>(marker::PhantomData<fn() -> I>);

// TODO: Need to forward supertrait raw control flow to downstream trait impl

/// A trait that describes an *Interrupt Service Routine*.
///
/// ## Multiple Entrance Vectors
///
/// This *Interrupt Service Routine* can be entered from many distinct vectors.
///
/// # Safety
///
/// Implementors must preserve the processor interrupt-entry ABI for every
/// implemented vector. The raw entry point must use the expected stack layout,
/// preserve all required register state, and never return normally.
pub unsafe trait Routine<const N: u8>
where
    Vector<N>: Available,
{
    /// The input to this *Interrupt Service Routine*.
    type Input;

    /// The *environmental constraint* imposed to this *Interrupt Service
    /// Routine*.
    ///
    /// That is, whether the *Interrupt Service Routine* expects a *Trap* or
    /// *Interrupt* *Gate Type*.
    type Mode: Environmental;

    /// The forwarder for the [`Routine::raw`] trampoline.
    type To: Forward;

    /// The *raw routine* for this *Interrupt Service Routine*.
    ///
    /// # Safety
    ///
    /// This associated trait function must be a *naked function*, (i.e., have
    /// no epilogue or prologue).
    ///
    /// Note that the `extern "C"` part is a lie, this follows the regular
    /// Interrupt Stack Layout, on top of the [`trampoline`] entry vector.
    ///
    /// ## Interrupt Entry Stack Layout
    ///
    /// On jump to the corresponding [`trampoline<N, Self>`], on top of the
    /// regular architecture-specific interrupt layout, the vector number is
    /// pushed.
    ///
    /// # Remarks
    ///
    /// This is implemented by-default. Overriding the default implementation is
    /// possible yet discouraged.
    // TODO: We need to write our entry functions manually in global_asm
    #[doc(hidden/* reason = "this function must be a last resort" */)]
    #[unsafe(naked)]
    unsafe extern "C" fn raw(target_service: &Service) -> ! {
        arch::naked_asm!(
            "jmp {}",
            "ud2",
            sym <Self::To as Forward>::raw,
            options(att_syntax)
        )
    }

    /// The actual routine used in the *Interrupt Service Routine*.
    ///
    /// # Safety
    ///
    /// The service reference and input must describe the active interrupt
    /// frame for vector `N`. The implementation must restore or transfer that
    /// frame according to the processor interrupt-entry ABI and must not
    /// return normally.
    unsafe fn me(target_service: &Service, target_input: Self::Input) -> !;
}

/// An *Interrupt Gate* to a *Interrupt Service [`Routine`]*.
///
/// # Safety
///
/// Implementors must satisfy [`Routine`] and must only be installed in an
/// interrupt gate whose entry disables maskable interrupts.
pub unsafe trait Gate<const N: u8>: Routine<N, Mode = Interrupted>
where
    Vector<N>: Available,
{
}

/// An *Trap Gate* to a *Interrupt Service [`Routine`]*.
///
/// # Safety
///
/// Implementors must satisfy [`Routine`] and must only be installed in a trap
/// gate whose entry preserves the interrupt-enable state.
pub unsafe trait Trap<const N: u8>: Routine<N, Mode = Trapped>
where
    Vector<N>: Available,
{
}

/// A trampoline to a (`N`, [`Routine`]) 2-tuple describing an *Interrupt
/// Service [`Routine`]*.
///
/// This is the *true entry to an ISR*. This trampoline pushes the const-generic
/// `N` to the stack for further inspection or distinction of the interrupt
/// vector in case where the routine is shared across many distinct
/// [`Routine`]s.
#[doc(hidden/* reason = "this function must be a last resort" */)]
#[unsafe(naked)]
pub unsafe extern "C" fn trampoline<const N: u8, R>() -> !
where
    R: Routine<N>,
    Vector<N>: Available,
{
    arch::naked_asm!(
        "pushb {interrupt_vector}",
        "jmp {interrupt_handler}",
        "ud2",
        interrupt_vector = const N,
        interrupt_handler = sym <R as Routine<N>>::raw,
        options(att_syntax)
    )
}
