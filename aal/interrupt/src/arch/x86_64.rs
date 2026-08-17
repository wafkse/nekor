//! Tightly-packed raw versions of interrupt structures.

use core::{arch, hash, marker, mem, num::NonZeroU32};

use nekor_bitwise::prelude::{Bit, Counterpart, Field, State};

use nekor_aal_agnostic::{partitioned::Partitioned, reserved::Reserved};

use nekor_aal_arch::{
    x86::{privilege::PrivilegeLevel, segmentation::RawCodeSegment},
    x86_64::gate::GateType64,
};

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
pub type TypeAttributesPrivilegeLevelMut<'a> =
    <TypeAttributesPrivilegeLevel<'a> as Counterpart>::Mut;

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
/// The bit layout of this type is exact to the one expected by the CPU.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct RawTypeAttributes(u8);

impl RawTypeAttributes {
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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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

        target_attributes
            .gate_type_mut()
            .const_merge(gate_type as _);

        target_attributes
            .privilege_level_mut()
            .const_merge(privilege_level as _);

        let target_state = if matches!(is_present, Present::Yes) {
            State::Set
        } else {
            State::Cleared
        };

        target_attributes.present_mut().const_set(target_state);

        target_attributes
    }
}

/// A 64-bit raw *Gate Descriptor* structure.
///
/// Reference: <https://wiki.osdev.org/Interrupt_Descriptor_Table#Structure_on_x86-64>
#[derive(Debug, Clone, Copy)]
#[repr(
    C,
    // NOTE(alignment): This requires 8-byte alignment.
    align(8))
]
pub struct RawGateDescriptor {
    /// The bits `0..=15` of the interrupt handler address.
    pub offset_0_15: Partitioned<0, 15, u64>,

    /// The raw *Code Segment Selector* used for this interrupt gate.
    pub segment: RawCodeSegment,

    /// An alternative stack pointer for the handler of this interrupt gate.
    ///
    /// When [`None`], the stack pointer is left as-is.
    // NOTE(layout): This is technically a 3-bit field but it is defined as an
    // 8-bit enumeration with zeroed upper variant bits. However, underlying
    // layout still matches as expected by the processor.
    pub ist: Option<RawIst>,

    /// The type attributes of this *Gate Descriptor*.
    pub type_attributes: RawTypeAttributes,

    /// The bits `16..=31` of the interrupt handler address.
    pub offset_16_31: Partitioned<16, 31, u64>,

    /// The bits `32..=31` of the interrupt handler address.
    pub offset_32_63: Partitioned<32, 63, u64>,

    /// These bits are reserved.
    reserved0: Reserved<u32>,
}

impl RawGateDescriptor {
    /// Determine the *Code Segment Descriptor* that is part of this
    /// [`RawGateDescriptor`].
    #[inline]
    #[must_use]
    pub const fn segment(&self) -> &RawCodeSegment {
        let Self { segment, .. } = self;

        segment
    }

    /// Determine the *Code Segment Descriptor* that is part of this
    /// [`RawGateDescriptor`].
    #[inline]
    pub const fn segment_mut(&mut self) -> &mut RawCodeSegment {
        let &mut Self {
            ref mut segment, ..
        } = self;

        segment
    }

    /// Determine the bits `0..=15` of the interrupt handler address.
    #[inline]
    #[must_use]
    pub const fn offset_0_15(&self) -> &Partitioned<0, 15, u64> {
        let Self { offset_0_15, .. } = self;

        offset_0_15
    }

    /// Determine the mutable reference to the bits `0..=15` of the interrupt
    /// handler address.
    #[inline]
    pub const fn offset_0_15_mut(&mut self) -> &mut Partitioned<0, 15, u64> {
        let &mut Self {
            ref mut offset_0_15,
            ..
        } = self;

        offset_0_15
    }

    /// Determine the bits `16..=31` of the interrupt handler address.
    #[inline]
    #[must_use]
    pub const fn offset_16_31(&self) -> &Partitioned<16, 31, u64> {
        let Self { offset_16_31, .. } = self;

        offset_16_31
    }

    /// Determine the mutable reference to the bits `16..=31` of the interrupt
    /// handler address.
    #[inline]
    pub const fn offset_16_31_mut(&mut self) -> &mut Partitioned<16, 31, u64> {
        let &mut Self {
            ref mut offset_16_31,
            ..
        } = self;

        offset_16_31
    }

    /// Determine the bits `32..=63` of the interrupt handler address.
    #[inline]
    #[must_use]
    pub const fn offset_32_63(&self) -> &Partitioned<32, 63, u64> {
        let Self { offset_32_63, .. } = self;

        offset_32_63
    }

    /// Determine the mutable reference to the bits `32..=63` of the interrupt
    /// handler address.
    #[inline]
    pub const fn offset_32_63_mut(&mut self) -> &mut Partitioned<32, 63, u64> {
        let &mut Self {
            ref mut offset_32_63,
            ..
        } = self;

        offset_32_63
    }

    /// Determine the IST (Interrupt Stack Table) field.
    #[inline]
    #[must_use]
    pub const fn interrupt_stack_table(&self) -> Option<RawIst> {
        let &Self { ist, .. } = self;

        ist
    }

    /// Determine the mutable reference to the IST (Interrupt Stack Table)
    /// field.
    #[inline]
    pub const fn interrupt_stack_table_mut(&mut self) -> &mut Option<RawIst> {
        let &mut Self { ref mut ist, .. } = self;

        ist
    }

    /// Determine the type attributes of this *Gate Descriptor*.
    #[inline]
    #[must_use]
    pub const fn type_attributes(&self) -> &RawTypeAttributes {
        let Self {
            type_attributes, ..
        } = self;

        type_attributes
    }

    /// Determine the mutable reference to the type attributes of this *Gate
    /// Descriptor*.
    #[inline]
    pub const fn type_attributes_mut(&mut self) -> &mut RawTypeAttributes {
        let &mut Self {
            ref mut type_attributes,
            ..
        } = self;

        type_attributes
    }
}

/// Assert that the this *Gate Descriptor* attribute is 16-bytes long and is
/// aligned to a 8-byte boundary.
const _: () = {
    assert!(mem::size_of::<RawGateDescriptor>() == mem::size_of::<u128>());

    assert!(mem::align_of::<RawGateDescriptor>() == mem::align_of::<usize>());
};

impl Eq for RawGateDescriptor {}

impl PartialEq for RawGateDescriptor {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let &Self {
            offset_0_15,
            segment,
            ist,
            type_attributes,
            offset_16_31,
            offset_32_63,
            ..
        } = self;

        let &Self {
            offset_0_15: other_offset_0_15,
            segment: other_segment,
            ist: other_ist,
            type_attributes: other_type_attributes,
            offset_16_31: other_offset_16_31,
            offset_32_63: other_offset_32_63,
            ..
        } = other;

        offset_0_15 == other_offset_0_15
            && segment == other_segment
            && ist == other_ist
            && type_attributes == other_type_attributes
            && offset_16_31 == other_offset_16_31
            && offset_32_63 == other_offset_32_63
    }
}

impl hash::Hash for RawGateDescriptor {
    #[inline]
    fn hash<H: hash::Hasher>(&self, target_state: &mut H) {
        let &Self {
            offset_0_15,
            segment,
            ist,
            type_attributes,
            offset_16_31,
            offset_32_63,
            ..
        } = self;

        offset_0_15.hash(target_state);

        segment.hash(target_state);

        ist.hash(target_state);

        type_attributes.hash(target_state);

        offset_16_31.hash(target_state);

        offset_32_63.hash(target_state);
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
    ///     - Genuine to the standard stack layout and in an actual
    ///       processor-invoked interrupt.
    ///     - In the same format as a genuine one, within the guarantees of
    ///       non-reentrancy<sup>1</sup> for the specific interrupt vector and
    ///       gate type.
    ///
    /// [1]: Does not apply to *non-maskable interrupts* when in a *pseudo-interrupt* (forged or otherwise simulated) context.
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
        arch::naked_asm!("ud2")
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
            sym <Self::To as Forward>::raw
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
