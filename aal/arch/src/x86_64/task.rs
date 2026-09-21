//! Long-mode task-state and task-register representations.

use core::mem;

use nekor_aal_agnostic::partitioned::Partitioned;
use nekor_bitwise::prelude::State;
use zerocopy::{Immutable, IntoBytes};

use crate::{
    x86::{
        descriptor::DescriptorIndex,
        privilege::PrivilegeLevel,
        segmentation::{RawSegmentSelector, SegmentSelector, TableIndicator},
    },
    x86_64::paging::{La, La48, La57},
};

/// A selector proven suitable for the long-mode task register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The selector always names a non-null GDT entry at ring zero.
// The caller installing the GDT remains responsible for placing an available
// long-mode TSS descriptor at that entry.
pub struct TaskRegisterSelector(SegmentSelector);

impl TaskRegisterSelector {
    /// Lifts a raw selector image into a task-register selector.
    ///
    /// Returns `None` unless the image selects a non-null GDT entry at ring
    /// zero and its descriptor index fits the architectural representation.
    #[inline]
    #[must_use]
    pub const fn lift(target_value: u16) -> Option<Self> {
        let target_value = RawSegmentSelector::new(target_value);
        let requested_privilege = PrivilegeLevel::lift(target_value.requested_privilege_level().const_value());
        let table = target_value.table_indicator().const_state();
        let index = DescriptorIndex::lift(target_value.index().const_value());

        match (requested_privilege, table, index) {
            (Some(PrivilegeLevel::Ring0), State::Cleared, Some(index)) => Some(Self::new(index)),
            _ => None,
        }
    }

    /// Constructs a task-register selector for a GDT descriptor index.
    #[inline]
    #[must_use]
    pub const fn new(target_index: DescriptorIndex) -> Self {
        let selector = SegmentSelector::new(target_index, TableIndicator::Gdt, PrivilegeLevel::Ring0);

        Self(selector)
    }

    /// Returns the descriptor-table index selected by this task register.
    #[inline]
    #[must_use]
    pub const fn index(self) -> DescriptorIndex {
        let Self(selector) = self;

        *selector.index()
    }

    /// Returns the architectural selector image consumed by `LTR`.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u16 {
        let Self(selector) = self;

        selector.raw().raw()
    }
}

/// Low thirty-two bits of a task-state stack-pointer image.
pub type TaskStackLow = Partitioned<0, 31, u64>;

/// High thirty-two bits of a task-state stack-pointer image.
pub type TaskStackHigh = Partitioned<32, 63, u64>;

/// Raw fields for a privilege-transition stack pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoBytes, Immutable)]
#[repr(C)]
// NOTE(invariant): The two partitioned fields preserve the architectural low
// and high words while retaining the TSS's four-byte field alignment.
pub struct RawPrivilegeStack(TaskStackLow, TaskStackHigh);

impl RawPrivilegeStack {
    /// Constructs a unconstrained raw privilege-stack image.
    #[inline]
    #[must_use]
    pub const fn new(low: TaskStackLow, high: TaskStackHigh) -> Self {
        Self(low, high)
    }

    /// Returns the low architectural word.
    #[inline]
    #[must_use]
    pub const fn low(&self) -> &TaskStackLow {
        let &Self(ref low, ..) = self;

        low
    }

    /// Returns mutable access to the low architectural word.
    #[inline]
    pub const fn low_mut(&mut self) -> &mut TaskStackLow {
        let &mut Self(ref mut low, ..) = self;

        low
    }

    /// Returns the high architectural word.
    #[inline]
    #[must_use]
    pub const fn high(&self) -> &TaskStackHigh {
        let &Self(_, ref high) = self;

        high
    }

    /// Returns mutable access to the high architectural word.
    #[inline]
    pub const fn high_mut(&mut self) -> &mut TaskStackHigh {
        let &mut Self(_, ref mut high) = self;

        high
    }

    /// Lowers a canonical address into the architectural stack fields.
    #[inline]
    #[must_use]
    fn lower(pointer: La) -> Self {
        let target_value = pointer.bits();
        let low = TaskStackLow::away(target_value);
        let high = TaskStackHigh::away(target_value);

        Self(low, high)
    }

    /// Lifts the architectural stack fields into a canonical address.
    #[inline]
    #[must_use]
    pub fn lift(self) -> Option<La> {
        let Self(low, high) = self;

        let mut target_value = u64::MIN;

        let _: u64 = low.merge(&mut target_value);

        let _: u64 = high.merge(&mut target_value);

        match La::new::<La48>(target_value) {
            Some(pointer) => Some(pointer),
            None => La::new::<La57>(target_value),
        }
    }
}

/// Raw fields for an interrupt-stack-table pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoBytes, Immutable)]
#[repr(C)]
// NOTE(invariant): The two partitioned fields preserve the architectural low
// and high words while retaining the TSS's four-byte field alignment.
pub struct RawInterruptStack(TaskStackLow, TaskStackHigh);

impl RawInterruptStack {
    /// Constructs a unconstrained raw interrupt-stack image.
    #[inline]
    #[must_use]
    pub const fn new(low: TaskStackLow, high: TaskStackHigh) -> Self {
        Self(low, high)
    }

    /// Returns the low architectural word.
    #[inline]
    #[must_use]
    pub const fn low(&self) -> &TaskStackLow {
        let &Self(ref low, ..) = self;

        low
    }

    /// Returns mutable access to the low architectural word.
    #[inline]
    pub const fn low_mut(&mut self) -> &mut TaskStackLow {
        let &mut Self(ref mut low, ..) = self;

        low
    }

    /// Returns the high architectural word.
    #[inline]
    #[must_use]
    pub const fn high(&self) -> &TaskStackHigh {
        let &Self(_, ref high) = self;

        high
    }

    /// Returns mutable access to the high architectural word.
    #[inline]
    pub const fn high_mut(&mut self) -> &mut TaskStackHigh {
        let &mut Self(_, ref mut high) = self;

        high
    }

    /// Lowers a canonical address into the architectural stack fields.
    #[inline]
    #[must_use]
    fn lower(pointer: La) -> Self {
        let target_value = pointer.bits();
        let low = TaskStackLow::away(target_value);
        let high = TaskStackHigh::away(target_value);

        Self(low, high)
    }

    /// Lifts the architectural stack fields into a canonical address.
    #[inline]
    #[must_use]
    pub fn lift(self) -> Option<La> {
        let Self(low, high) = self;

        let mut target_value = u64::MIN;

        let _: u64 = low.merge(&mut target_value);

        let _: u64 = high.merge(&mut target_value);

        match La::new::<La48>(target_value) {
            Some(pointer) => Some(pointer),
            None => La::new::<La57>(target_value),
        }
    }
}

/// The I/O permission-map byte offset stored in a task-state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoBytes, Immutable)]
#[repr(transparent)]
// NOTE(invariant): Every `u16` is a representable raw architectural offset.
// Semantic task-state lowering uses the exact fixed-base size.
pub struct TaskIoMapBase(u16);

impl TaskIoMapBase {
    /// Constructs the architectural I/O map byte offset.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u16) -> Self {
        Self(target_value)
    }

    /// Returns the architectural offset image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u16 {
        let Self(target_value) = self;

        target_value
    }
}

/// The fixed 104-byte raw long-mode task-state image.
#[derive(Debug, Clone, Copy, IntoBytes, Immutable)]
#[repr(C)]
// NOTE(invariant): Field order and four-byte alignment form the exact fixed
// long-mode TSS image. Values produced by `TaskStateSegment::raw` have zeroed
// reserved fields, canonical stack addresses, and an I/O map base of 104.
pub struct RawTaskStateSegmentBase {
    /// Reserved initial word.
    reserved_header: u32,

    /// Stack loaded for a transition to CPL0.
    rsp0: RawPrivilegeStack,

    /// Stack loaded for a transition to CPL1.
    rsp1: RawPrivilegeStack,

    /// Stack loaded for a transition to CPL2.
    rsp2: RawPrivilegeStack,

    /// Reserved words after the privilege-transition stacks.
    reserved_privilege: [u32; 2],

    /// First interrupt-stack-table entry, architecturally IST1.
    ist0: RawInterruptStack,

    /// Second interrupt-stack-table entry, architecturally IST2.
    ist1: RawInterruptStack,

    /// Third interrupt-stack-table entry, architecturally IST3.
    ist2: RawInterruptStack,

    /// Fourth interrupt-stack-table entry, architecturally IST4.
    ist3: RawInterruptStack,

    /// Fifth interrupt-stack-table entry, architecturally IST5.
    ist4: RawInterruptStack,

    /// Sixth interrupt-stack-table entry, architecturally IST6.
    ist5: RawInterruptStack,

    /// Seventh interrupt-stack-table entry, architecturally IST7.
    ist6: RawInterruptStack,

    /// Reserved words after the interrupt-stack-table entries.
    reserved_interrupt: [u32; 2],

    /// Reserved word before the I/O map base field.
    reserved_io: u16,

    /// Byte offset from the TSS base to the trailing I/O map area.
    io_map_base: TaskIoMapBase,
}

impl RawTaskStateSegmentBase {
    /// Constructs a fixed raw task-state image.
    #[expect(
        clippy::too_many_arguments,
        reason = "the constructor names every fixed architectural TSS field"
    )]
    #[inline]
    #[must_use]
    pub const fn new(
        reserved_header: u32,
        rsp0: RawPrivilegeStack,
        rsp1: RawPrivilegeStack,
        rsp2: RawPrivilegeStack,
        reserved_privilege: [u32; 2],
        ist0: RawInterruptStack,
        ist1: RawInterruptStack,
        ist2: RawInterruptStack,
        ist3: RawInterruptStack,
        ist4: RawInterruptStack,
        ist5: RawInterruptStack,
        ist6: RawInterruptStack,
        reserved_interrupt: [u32; 2],
        reserved_io: u16,
        io_map_base: TaskIoMapBase,
    ) -> Self {
        Self {
            reserved_header,
            rsp0,
            rsp1,
            rsp2,
            reserved_privilege,
            ist0,
            ist1,
            ist2,
            ist3,
            ist4,
            ist5,
            ist6,
            reserved_interrupt,
            reserved_io,
            io_map_base,
        }
    }

    /// Lowers the semantic task state into its fixed architectural image.
    #[inline]
    #[must_use]
    fn lower<const N: usize>(state: &TaskStateSegment<N>) -> Self {
        let &TaskStateSegment {
            rsp0,
            rsp1,
            rsp2,
            ist0,
            ist1,
            ist2,
            ist3,
            ist4,
            ist5,
            ist6,
            ..
        } = state;

        let reserved_header = 0;
        let rsp0 = RawPrivilegeStack::lower(rsp0);
        let rsp1 = RawPrivilegeStack::lower(rsp1);
        let rsp2 = RawPrivilegeStack::lower(rsp2);
        let reserved_privilege = [0; 2];
        let ist0 = RawInterruptStack::lower(ist0);
        let ist1 = RawInterruptStack::lower(ist1);
        let ist2 = RawInterruptStack::lower(ist2);
        let ist3 = RawInterruptStack::lower(ist3);
        let ist4 = RawInterruptStack::lower(ist4);
        let ist5 = RawInterruptStack::lower(ist5);
        let ist6 = RawInterruptStack::lower(ist6);
        let reserved_interrupt = [0; 2];
        let reserved_io = 0;
        let io_map_base = TaskIoMapBase::new(mem::size_of::<Self>() as u16);

        Self {
            reserved_header,
            rsp0,
            rsp1,
            rsp2,
            reserved_privilege,
            ist0,
            ist1,
            ist2,
            ist3,
            ist4,
            ist5,
            ist6,
            reserved_interrupt,
            reserved_io,
            io_map_base,
        }
    }

    /// Returns the raw stack loaded for a transition to CPL0.
    #[inline]
    #[must_use]
    pub const fn rsp0(&self) -> &RawPrivilegeStack {
        let &Self { ref rsp0, .. } = self;

        rsp0
    }

    /// Returns mutable access to the raw stack loaded for a transition to CPL0.
    #[inline]
    pub const fn rsp0_mut(&mut self) -> &mut RawPrivilegeStack {
        let &mut Self { ref mut rsp0, .. } = self;

        rsp0
    }

    /// Returns the raw stack loaded for a transition to CPL1.
    #[inline]
    #[must_use]
    pub const fn rsp1(&self) -> &RawPrivilegeStack {
        let &Self { ref rsp1, .. } = self;

        rsp1
    }

    /// Returns mutable access to the raw stack loaded for a transition to CPL1.
    #[inline]
    pub const fn rsp1_mut(&mut self) -> &mut RawPrivilegeStack {
        let &mut Self { ref mut rsp1, .. } = self;

        rsp1
    }

    /// Returns the raw stack loaded for a transition to CPL2.
    #[inline]
    #[must_use]
    pub const fn rsp2(&self) -> &RawPrivilegeStack {
        let &Self { ref rsp2, .. } = self;

        rsp2
    }

    /// Returns mutable access to the raw stack loaded for a transition to CPL2.
    #[inline]
    pub const fn rsp2_mut(&mut self) -> &mut RawPrivilegeStack {
        let &mut Self { ref mut rsp2, .. } = self;

        rsp2
    }

    /// Returns the first raw interrupt-stack-table entry, IST1.
    #[inline]
    #[must_use]
    pub const fn ist0(&self) -> &RawInterruptStack {
        let &Self { ref ist0, .. } = self;

        ist0
    }

    /// Returns mutable access to the first raw interrupt-stack-table entry, IST1.
    #[inline]
    pub const fn ist0_mut(&mut self) -> &mut RawInterruptStack {
        let &mut Self { ref mut ist0, .. } = self;

        ist0
    }

    /// Returns the second raw interrupt-stack-table entry, IST2.
    #[inline]
    #[must_use]
    pub const fn ist1(&self) -> &RawInterruptStack {
        let &Self { ref ist1, .. } = self;

        ist1
    }

    /// Returns mutable access to the second raw interrupt-stack-table entry, IST2.
    #[inline]
    pub const fn ist1_mut(&mut self) -> &mut RawInterruptStack {
        let &mut Self { ref mut ist1, .. } = self;

        ist1
    }

    /// Returns the third raw interrupt-stack-table entry, IST3.
    #[inline]
    #[must_use]
    pub const fn ist2(&self) -> &RawInterruptStack {
        let &Self { ref ist2, .. } = self;

        ist2
    }

    /// Returns mutable access to the third raw interrupt-stack-table entry, IST3.
    #[inline]
    pub const fn ist2_mut(&mut self) -> &mut RawInterruptStack {
        let &mut Self { ref mut ist2, .. } = self;

        ist2
    }

    /// Returns the fourth raw interrupt-stack-table entry, IST4.
    #[inline]
    #[must_use]
    pub const fn ist3(&self) -> &RawInterruptStack {
        let &Self { ref ist3, .. } = self;

        ist3
    }

    /// Returns mutable access to the fourth raw interrupt-stack-table entry, IST4.
    #[inline]
    pub const fn ist3_mut(&mut self) -> &mut RawInterruptStack {
        let &mut Self { ref mut ist3, .. } = self;

        ist3
    }

    /// Returns the fifth raw interrupt-stack-table entry, IST5.
    #[inline]
    #[must_use]
    pub const fn ist4(&self) -> &RawInterruptStack {
        let &Self { ref ist4, .. } = self;

        ist4
    }

    /// Returns mutable access to the fifth raw interrupt-stack-table entry, IST5.
    #[inline]
    pub const fn ist4_mut(&mut self) -> &mut RawInterruptStack {
        let &mut Self { ref mut ist4, .. } = self;

        ist4
    }

    /// Returns the sixth raw interrupt-stack-table entry, IST6.
    #[inline]
    #[must_use]
    pub const fn ist5(&self) -> &RawInterruptStack {
        let &Self { ref ist5, .. } = self;

        ist5
    }

    /// Returns mutable access to the sixth raw interrupt-stack-table entry, IST6.
    #[inline]
    pub const fn ist5_mut(&mut self) -> &mut RawInterruptStack {
        let &mut Self { ref mut ist5, .. } = self;

        ist5
    }

    /// Returns the seventh raw interrupt-stack-table entry, IST7.
    #[inline]
    #[must_use]
    pub const fn ist6(&self) -> &RawInterruptStack {
        let &Self { ref ist6, .. } = self;

        ist6
    }

    /// Returns mutable access to the seventh raw interrupt-stack-table entry, IST7.
    #[inline]
    pub const fn ist6_mut(&mut self) -> &mut RawInterruptStack {
        let &mut Self { ref mut ist6, .. } = self;

        ist6
    }

    /// Returns the byte offset of the trailing I/O permission map.
    #[inline]
    #[must_use]
    pub const fn io_map_base(&self) -> TaskIoMapBase {
        let &Self { io_map_base, .. } = self;

        io_map_base
    }

    /// Returns mutable access to the trailing I/O permission-map offset.
    #[inline]
    pub const fn io_map_base_mut(&mut self) -> &mut TaskIoMapBase {
        let &mut Self {
            ref mut io_map_base, ..
        } = self;

        io_map_base
    }
}

/// Raw bytes of an x86 I/O permission map including its terminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoBytes, Immutable)]
#[repr(transparent)]
// NOTE(invariant): The array is the complete trailing architectural image. Raw
// construction deliberately permits a missing or malformed terminator.
pub struct RawIoPermissionMap<const N: usize = 0>([u8; N]);

impl<const N: usize> RawIoPermissionMap<N> {
    /// Constructs a raw I/O permission-map image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: [u8; N]) -> Self {
        Self(target_value)
    }

    /// Returns the raw map image.
    #[inline]
    #[must_use]
    pub const fn raw(&self) -> &[u8; N] {
        let &Self(ref target_value) = self;

        target_value
    }

    /// Returns mutable access to the raw map image.
    #[inline]
    pub const fn raw_mut(&mut self) -> &mut [u8; N] {
        let &mut Self(ref mut target_value) = self;

        target_value
    }
}

/// An x86 I/O port number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Every u16 is a complete architectural x86 I/O-port number.
pub struct IoPort(u16);

impl IoPort {
    /// Constructs an I/O port from its architectural number.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u16) -> Self {
        Self(target_value)
    }

    /// Returns the architectural port number.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u16 {
        let Self(target_value) = self;

        target_value
    }
}

/// Permission assigned to an I/O port by a task-state I/O map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IoPermission {
    /// The task may access the port when the I/O privilege checks reach the map.
    Permitted,

    /// The task may not access the port when the I/O privilege checks reach the map.
    Denied,
}

/// A checked x86 I/O permission map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
// NOTE(invariant): Empty storage means no map. Nonempty storage always ends in
// an all-ones terminator, and safe mutation can change only represented port bits.
pub struct IoPermissionMap<const N: usize = 0>(RawIoPermissionMap<N>);

impl<const N: usize> IoPermissionMap<N> {
    /// Constructs a map that denies every represented port.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(RawIoPermissionMap::new([u8::MAX; N]))
    }

    /// Lifts a raw map when it is empty or has the required terminator.
    #[inline]
    #[must_use]
    pub const fn lift(target_value: RawIoPermissionMap<N>) -> Option<Self> {
        match target_value.raw().last() {
            Some(&u8::MAX) | None => Some(Self(target_value)),
            Some(_) => None,
        }
    }

    /// Returns the number of ports represented by this map.
    #[inline]
    #[must_use]
    pub const fn capacity(&self) -> usize {
        N.saturating_sub(1).saturating_mul(u8::BITS as usize)
    }

    /// Returns the permission assigned to a represented port.
    #[inline]
    #[must_use]
    pub fn permission(&self, port: IoPort) -> Option<IoPermission> {
        let &Self(ref raw) = self;

        let target_port = port.raw() as usize;
        let byte_index = target_port / u8::BITS as usize;
        let bit_index = target_port % u8::BITS as usize;

        match (byte_index < N.saturating_sub(1), raw.raw().get(byte_index)) {
            (true, Some(target_byte)) => match target_byte & (1 << bit_index) {
                0 => Some(IoPermission::Permitted),
                _ => Some(IoPermission::Denied),
            },
            _ => None,
        }
    }

    /// Permits access to a represented port.
    #[inline]
    pub fn permit(&mut self, port: IoPort) -> Option<()> {
        self.replace(port, IoPermission::Permitted)
    }

    /// Denies access to a represented port.
    #[inline]
    pub fn deny(&mut self, port: IoPort) -> Option<()> {
        self.replace(port, IoPermission::Denied)
    }

    /// Returns the checked map's raw architectural image.
    #[inline]
    #[must_use]
    pub const fn raw(&self) -> &RawIoPermissionMap<N> {
        let &Self(ref raw) = self;

        raw
    }

    /// Replaces the permission assigned to a represented port.
    #[inline]
    fn replace(&mut self, port: IoPort, permission: IoPermission) -> Option<()> {
        let &mut Self(RawIoPermissionMap(ref mut target_value)) = self;

        let target_port = port.raw() as usize;
        let byte_index = target_port / u8::BITS as usize;
        let bit_index = target_port % u8::BITS as usize;
        let target_byte = target_value.get_mut(byte_index);

        match (byte_index < N.saturating_sub(1), target_byte, permission) {
            (true, Some(target_byte), IoPermission::Permitted) => {
                *target_byte &= !(1 << bit_index);

                Some(())
            },
            (true, Some(target_byte), IoPermission::Denied) => {
                *target_byte |= 1 << bit_index;

                Some(())
            },
            _ => None,
        }
    }
}

impl<const N: usize> Default for IoPermissionMap<N> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// A raw long-mode task-state area with a trailing I/O map image.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
// NOTE(invariant): The map starts immediately after the fixed base. The
// architectural limit excludes any Rust tail padding after the map.
pub struct RawTaskStateSegment<const N: usize = 0>(RawTaskStateSegmentBase, RawIoPermissionMap<N>);

impl<const N: usize> RawTaskStateSegment<N> {
    /// Returns the fixed task-state image.
    #[inline]
    #[must_use]
    pub const fn base(&self) -> &RawTaskStateSegmentBase {
        let &Self(ref base, ..) = self;

        base
    }

    /// Returns mutable access to the fixed task-state image.
    #[inline]
    pub const fn base_mut(&mut self) -> &mut RawTaskStateSegmentBase {
        let &mut Self(ref mut base, ..) = self;

        base
    }

    /// Returns the trailing raw I/O permission map.
    #[inline]
    #[must_use]
    pub const fn io_map(&self) -> &RawIoPermissionMap<N> {
        let &Self(_, ref io_map) = self;

        io_map
    }

    /// Returns mutable access to the trailing raw I/O permission map.
    #[inline]
    pub const fn io_map_mut(&mut self) -> &mut RawIoPermissionMap<N> {
        let &mut Self(_, ref mut io_map) = self;

        io_map
    }

    /// Returns the architectural byte limit for this task-state area.
    #[inline]
    #[must_use]
    pub const fn limit(&self) -> Option<u64> {
        const MAX_LIMIT: usize = (1 << 20) - 1;

        let byte_length = mem::size_of::<RawTaskStateSegmentBase>() + N;
        // NOTE: RawTaskStateSegmentBase has nonzero size, so the complete
        // task-state area is always at least one byte and the one-biased limit cannot underflow.
        let target_value = byte_length - 1;

        match target_value {
            0..=MAX_LIMIT => Some(target_value as u64),
            _ => None,
        }
    }
}

/// A semantic long-mode task-state segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// NOTE(invariant): Every stack is a canonical linear address and the checked
// I/O map preserves its terminator. Raw lowering supplies all reserved fields
// and architectural split fields without exposing those details to callers.
pub struct TaskStateSegment<const N: usize = 0> {
    /// Stack loaded for a transition to CPL0.
    rsp0: La,

    /// Stack loaded for a transition to CPL1.
    rsp1: La,

    /// Stack loaded for a transition to CPL2.
    rsp2: La,

    /// First interrupt-stack-table entry, architecturally IST1.
    ist0: La,

    /// Second interrupt-stack-table entry, architecturally IST2.
    ist1: La,

    /// Third interrupt-stack-table entry, architecturally IST3.
    ist2: La,

    /// Fourth interrupt-stack-table entry, architecturally IST4.
    ist3: La,

    /// Fifth interrupt-stack-table entry, architecturally IST5.
    ist4: La,

    /// Sixth interrupt-stack-table entry, architecturally IST6.
    ist5: La,

    /// Seventh interrupt-stack-table entry, architecturally IST7.
    ist6: La,

    /// Checked trailing I/O permission map.
    io_map: IoPermissionMap<N>,
}

impl<const N: usize> TaskStateSegment<N> {
    /// Constructs a zeroed task state that denies every represented I/O port.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        let zero = match La::new::<La48>(u64::MIN) {
            Some(zero) => zero,
            None => unreachable!(),
        };
        let io_map = IoPermissionMap::new();

        Self {
            rsp0: zero,
            rsp1: zero,
            rsp2: zero,
            ist0: zero,
            ist1: zero,
            ist2: zero,
            ist3: zero,
            ist4: zero,
            ist5: zero,
            ist6: zero,
            io_map,
        }
    }

    /// Lowers this semantic state into the architectural task-state area.
    #[inline]
    #[must_use]
    pub fn raw(&self) -> RawTaskStateSegment<N> {
        let &Self { io_map, .. } = self;

        let base = RawTaskStateSegmentBase::lower(self);
        let io_map = *io_map.raw();

        RawTaskStateSegment(base, io_map)
    }

    /// Returns the stack loaded for a transition to CPL0.
    #[inline]
    #[must_use]
    pub const fn rsp0(&self) -> &La {
        let &Self { ref rsp0, .. } = self;

        rsp0
    }

    /// Returns mutable access to the stack loaded for a transition to CPL0.
    #[inline]
    pub const fn rsp0_mut(&mut self) -> &mut La {
        let &mut Self { ref mut rsp0, .. } = self;

        rsp0
    }

    /// Returns the stack loaded for a transition to CPL1.
    #[inline]
    #[must_use]
    pub const fn rsp1(&self) -> &La {
        let &Self { ref rsp1, .. } = self;

        rsp1
    }

    /// Returns mutable access to the stack loaded for a transition to CPL1.
    #[inline]
    pub const fn rsp1_mut(&mut self) -> &mut La {
        let &mut Self { ref mut rsp1, .. } = self;

        rsp1
    }

    /// Returns the stack loaded for a transition to CPL2.
    #[inline]
    #[must_use]
    pub const fn rsp2(&self) -> &La {
        let &Self { ref rsp2, .. } = self;

        rsp2
    }

    /// Returns mutable access to the stack loaded for a transition to CPL2.
    #[inline]
    pub const fn rsp2_mut(&mut self) -> &mut La {
        let &mut Self { ref mut rsp2, .. } = self;

        rsp2
    }

    /// Returns the first interrupt-stack-table entry, IST1.
    #[inline]
    #[must_use]
    pub const fn ist0(&self) -> &La {
        let &Self { ref ist0, .. } = self;

        ist0
    }

    /// Returns mutable access to the first interrupt-stack-table entry, IST1.
    #[inline]
    pub const fn ist0_mut(&mut self) -> &mut La {
        let &mut Self { ref mut ist0, .. } = self;

        ist0
    }

    /// Returns the second interrupt-stack-table entry, IST2.
    #[inline]
    #[must_use]
    pub const fn ist1(&self) -> &La {
        let &Self { ref ist1, .. } = self;

        ist1
    }

    /// Returns mutable access to the second interrupt-stack-table entry, IST2.
    #[inline]
    pub const fn ist1_mut(&mut self) -> &mut La {
        let &mut Self { ref mut ist1, .. } = self;

        ist1
    }

    /// Returns the third interrupt-stack-table entry, IST3.
    #[inline]
    #[must_use]
    pub const fn ist2(&self) -> &La {
        let &Self { ref ist2, .. } = self;

        ist2
    }

    /// Returns mutable access to the third interrupt-stack-table entry, IST3.
    #[inline]
    pub const fn ist2_mut(&mut self) -> &mut La {
        let &mut Self { ref mut ist2, .. } = self;

        ist2
    }

    /// Returns the fourth interrupt-stack-table entry, IST4.
    #[inline]
    #[must_use]
    pub const fn ist3(&self) -> &La {
        let &Self { ref ist3, .. } = self;

        ist3
    }

    /// Returns mutable access to the fourth interrupt-stack-table entry, IST4.
    #[inline]
    pub const fn ist3_mut(&mut self) -> &mut La {
        let &mut Self { ref mut ist3, .. } = self;

        ist3
    }

    /// Returns the fifth interrupt-stack-table entry, IST5.
    #[inline]
    #[must_use]
    pub const fn ist4(&self) -> &La {
        let &Self { ref ist4, .. } = self;

        ist4
    }

    /// Returns mutable access to the fifth interrupt-stack-table entry, IST5.
    #[inline]
    pub const fn ist4_mut(&mut self) -> &mut La {
        let &mut Self { ref mut ist4, .. } = self;

        ist4
    }

    /// Returns the sixth interrupt-stack-table entry, IST6.
    #[inline]
    #[must_use]
    pub const fn ist5(&self) -> &La {
        let &Self { ref ist5, .. } = self;

        ist5
    }

    /// Returns mutable access to the sixth interrupt-stack-table entry, IST6.
    #[inline]
    pub const fn ist5_mut(&mut self) -> &mut La {
        let &mut Self { ref mut ist5, .. } = self;

        ist5
    }

    /// Returns the seventh interrupt-stack-table entry, IST7.
    #[inline]
    #[must_use]
    pub const fn ist6(&self) -> &La {
        let &Self { ref ist6, .. } = self;

        ist6
    }

    /// Returns mutable access to the seventh interrupt-stack-table entry, IST7.
    #[inline]
    pub const fn ist6_mut(&mut self) -> &mut La {
        let &mut Self { ref mut ist6, .. } = self;

        ist6
    }

    /// Returns the checked I/O permission map.
    #[inline]
    #[must_use]
    pub const fn io_map(&self) -> &IoPermissionMap<N> {
        let &Self { ref io_map, .. } = self;

        io_map
    }

    /// Returns mutable access to the checked I/O permission map.
    #[inline]
    pub const fn io_map_mut(&mut self) -> &mut IoPermissionMap<N> {
        let &mut Self { ref mut io_map, .. } = self;

        io_map
    }
}

impl<const N: usize> Default for TaskStateSegment<N> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{self, offset_of};

    use zerocopy::IntoBytes;

    use super::{
        IoPermission, IoPermissionMap, IoPort, RawInterruptStack, RawIoPermissionMap, RawPrivilegeStack,
        RawTaskStateSegment, RawTaskStateSegmentBase, TaskIoMapBase, TaskStackHigh, TaskStackLow, TaskStateSegment,
    };
    use crate::x86_64::paging::{La, La48, La57};

    #[test]
    fn raw_task_state_layout_and_stacks_round_trip() {
        let ring0 = La::new::<La48>(0x8000).expect("ring-zero stack must be canonical");
        let ist = La::new::<La48>(0x9000).expect("interrupt stack must be canonical");
        let mut state = TaskStateSegment::<0>::new();

        *state.rsp0_mut() = ring0;
        *state.ist0_mut() = ist;

        let raw = state.raw();
        let base = raw.base();

        assert_eq!(mem::size_of::<RawPrivilegeStack>(), 8);
        assert_eq!(mem::align_of::<RawPrivilegeStack>(), 4);
        assert_eq!(mem::size_of::<RawInterruptStack>(), 8);
        assert_eq!(mem::align_of::<RawInterruptStack>(), 4);
        assert_eq!(mem::size_of::<RawTaskStateSegmentBase>(), 104);
        assert_eq!(mem::align_of::<RawTaskStateSegmentBase>(), 4);
        assert_eq!(mem::size_of::<RawTaskStateSegment>(), 104);
        assert_eq!(mem::align_of::<RawTaskStateSegment>(), 4);
        assert_eq!(offset_of!(RawTaskStateSegmentBase, rsp0), 4);
        assert_eq!(offset_of!(RawTaskStateSegmentBase, ist0), 36);
        assert_eq!(offset_of!(RawTaskStateSegmentBase, io_map_base), 102);
        assert_eq!(base.rsp0().lift(), Some(ring0));
        assert_eq!(base.ist0().lift(), Some(ist));
        assert_eq!(base.io_map_base().raw(), 104);
    }

    #[test]
    fn raw_task_state_base_preserves_reserved_storage() {
        let stack = RawPrivilegeStack::new(TaskStackLow::raw(0), TaskStackHigh::raw(0));
        let interrupt = RawInterruptStack::new(TaskStackLow::raw(0), TaskStackHigh::raw(0));
        let raw = RawTaskStateSegmentBase::new(
            0x89ab_cdef,
            stack,
            stack,
            stack,
            [0x0123_4567, 0x7654_3210],
            interrupt,
            interrupt,
            interrupt,
            interrupt,
            interrupt,
            interrupt,
            interrupt,
            [0xa5a5_5a5a, 0x5a5a_a5a5],
            0xc3d4,
            TaskIoMapBase::new(104),
        );
        let bytes = raw.as_bytes();

        assert_eq!(bytes.len(), 104);
        assert_eq!(&bytes[0..4], &0x89ab_cdef_u32.to_ne_bytes());
        assert_eq!(&bytes[28..32], &0x0123_4567_u32.to_ne_bytes());
        assert_eq!(&bytes[32..36], &0x7654_3210_u32.to_ne_bytes());
        assert_eq!(&bytes[92..96], &0xa5a5_5a5a_u32.to_ne_bytes());
        assert_eq!(&bytes[96..100], &0x5a5a_a5a5_u32.to_ne_bytes());
        assert_eq!(&bytes[100..102], &0xc3d4_u16.to_ne_bytes());
    }

    #[test]
    fn io_map_extent_is_const_generic() {
        let state = TaskStateSegment::<3>::new();
        let raw = state.raw();

        assert_eq!(mem::size_of::<RawTaskStateSegment<3>>(), 108);
        assert_eq!(offset_of!(RawTaskStateSegment<3>, 1), 104);
        assert_eq!(raw.io_map().as_bytes(), &[u8::MAX; 3]);
        assert_eq!(raw.limit(), Some(106));
    }

    #[test]
    fn io_map_lift_requires_an_all_ones_terminator() {
        let absent = RawIoPermissionMap::new([]);
        let valid = RawIoPermissionMap::new([0, 0, u8::MAX]);
        let invalid = RawIoPermissionMap::new([0, 0, 0]);

        assert!(IoPermissionMap::lift(absent).is_some());
        assert!(IoPermissionMap::lift(valid).is_some());
        assert!(IoPermissionMap::lift(invalid).is_none());
    }

    #[test]
    fn io_map_uses_port_permissions_and_preserves_the_terminator() {
        let mut state = TaskStateSegment::<3>::new();
        let port = IoPort::new(9);

        assert_eq!(state.io_map().capacity(), 16);
        assert_eq!(state.io_map().permission(port), Some(IoPermission::Denied));
        assert_eq!(state.io_map_mut().permit(port), Some(()));
        assert_eq!(state.io_map().permission(port), Some(IoPermission::Permitted));
        assert_eq!(state.io_map_mut().deny(port), Some(()));
        assert_eq!(state.io_map().permission(port), Some(IoPermission::Denied));
        assert_eq!(state.io_map().permission(IoPort::new(16)), None);

        let raw = state.raw();

        assert_eq!(raw.io_map().raw().last(), Some(&u8::MAX));
    }

    #[test]
    fn la57_only_stack_pointer_round_trips() {
        let pointer = La::new::<La57>(0x0001_0000_0000_0000).expect("test pointer must be LA57 canonical");
        assert!(!pointer.is_canonical::<La48>());

        let mut state = TaskStateSegment::<0>::new();

        *state.rsp0_mut() = pointer;

        let raw = state.raw();

        assert_eq!(*state.rsp0(), pointer);
        assert_eq!(raw.base().rsp0().lift(), Some(pointer));
    }
}
