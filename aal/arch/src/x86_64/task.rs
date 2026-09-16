//! Long-mode task-state and task-register representations.

use core::mem;

use nekor_bitwise::prelude::{Counterpart, Field, State, U64High32, U64High32Mut, U64Low32, U64Low32Mut};

use crate::{
    x86::{
        descriptor::DescriptorIndex,
        privilege::PrivilegeLevel,
        segmentation::{RawSegmentSelector, SegmentSelector, TableIndicator},
    },
    x86_64::paging::{La, La48, La57},
};

/// I/O permission-map base field in the final task-state word.
type TaskIoMapBase<'value> = Field<'value, 16, 31, u32>;

/// Mutable counterpart to [`TaskIoMapBase`].
type TaskIoMapBaseMut<'value> = <TaskIoMapBase<'value> as Counterpart>::Mut;

/// One privilege-transition stack in a long-mode task-state segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrivilegeStack {
    /// Stack loaded when entering CPL0.
    Ring0,

    /// Stack loaded when entering CPL1.
    Ring1,

    /// Stack loaded when entering CPL2.
    Ring2,
}

/// One interrupt-stack-table slot in a long-mode task-state segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InterruptStack {
    /// Interrupt stack one.
    One,

    /// Interrupt stack two.
    Two,

    /// Interrupt stack three.
    Three,

    /// Interrupt stack four.
    Four,

    /// Interrupt stack five.
    Five,

    /// Interrupt stack six.
    Six,

    /// Interrupt stack seven.
    Seven,
}

/// A selector proven suitable for the long-mode task register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The selector always names a non-null GDT entry at ring zero.
// The caller installing the GDT remains responsible for placing an available
// long-mode TSS descriptor at that entry.
pub struct TaskRegisterSelector(SegmentSelector);

impl TaskRegisterSelector {
    /// Create a task-register selector from a raw selector image.
    ///
    /// Returns `None` unless the image selects a non-null GDT entry at ring
    /// zero and its descriptor index fits the architectural representation.
    #[inline]
    #[must_use]
    pub const fn from_raw(value: u16) -> Option<Self> {
        let raw = RawSegmentSelector::new(value);
        let requested = raw.requested_privilege_level().const_value();
        let requested = PrivilegeLevel::lift(requested);
        let gdt = matches!(raw.table_indicator().const_state(), State::Cleared);
        let ring_zero = matches!(requested, Some(PrivilegeLevel::Ring0));
        let index = DescriptorIndex::lift(raw.index().const_value());

        match (gdt, ring_zero, index) {
            (true, true, Some(index)) => Some(Self::new(index)),
            _ => None,
        }
    }

    /// Constructs a task-register selector for one GDT descriptor index.
    #[inline]
    #[must_use]
    pub const fn new(index: DescriptorIndex) -> Self {
        let selector = SegmentSelector::new(index, TableIndicator::Gdt, PrivilegeLevel::Ring0);

        Self(selector)
    }

    /// Returns the descriptor-table index selected by this task register.
    #[inline]
    #[must_use]
    pub const fn index(self) -> DescriptorIndex {
        let Self(selector) = self;
        let &index = selector.index();

        index
    }

    /// Returns the architectural selector image consumed by `LTR`.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u16 {
        let Self(selector) = self;

        selector.raw().get()
    }
}

/// One complete 64-bit task-state segment.
///
/// The storage uses explicit 32-bit words so its alignment and every reserved
/// offset remain exact without creating packed unaligned references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, align(4))]
// NOTE(invariant): The twenty-six words form the exact 104-byte long-mode TSS
// image. Stack fields are written only from canonical `La` values and the I/O
// map offset initially points one byte beyond the image to disable the bitmap.
pub struct TaskStateSegment([u32; 26]);

impl TaskStateSegment {
    /// Constructs an empty task-state segment with no I/O permission bitmap.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        let mut io_map_word = u32::MIN;
        let mut io_map_base = TaskIoMapBaseMut::wrap(&mut io_map_word);

        io_map_base.const_merge(mem::size_of::<Self>() as u16);

        let words = [
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            io_map_word,
        ];

        Self(words)
    }

    /// Sets one privilege-transition stack pointer.
    #[inline]
    pub fn set_privilege_stack(&mut self, stack: PrivilegeStack, pointer: La) {
        let index = match stack {
            PrivilegeStack::Ring0 => 1,
            PrivilegeStack::Ring1 => 3,
            PrivilegeStack::Ring2 => 5,
        };

        self.write_pointer(index, pointer);
    }

    /// Returns one privilege-transition stack pointer.
    #[inline]
    #[must_use]
    pub fn privilege_stack(&self, stack: PrivilegeStack) -> La {
        let index = match stack {
            PrivilegeStack::Ring0 => 1,
            PrivilegeStack::Ring1 => 3,
            PrivilegeStack::Ring2 => 5,
        };

        self.read_pointer(index)
    }

    /// Sets one interrupt-stack-table pointer.
    #[inline]
    pub fn set_interrupt_stack(&mut self, stack: InterruptStack, pointer: La) {
        let index = match stack {
            InterruptStack::One => 9,
            InterruptStack::Two => 11,
            InterruptStack::Three => 13,
            InterruptStack::Four => 15,
            InterruptStack::Five => 17,
            InterruptStack::Six => 19,
            InterruptStack::Seven => 21,
        };

        self.write_pointer(index, pointer);
    }

    /// Returns one interrupt-stack-table pointer.
    #[inline]
    #[must_use]
    pub fn interrupt_stack(&self, stack: InterruptStack) -> La {
        let index = match stack {
            InterruptStack::One => 9,
            InterruptStack::Two => 11,
            InterruptStack::Three => 13,
            InterruptStack::Four => 15,
            InterruptStack::Five => 17,
            InterruptStack::Six => 19,
            InterruptStack::Seven => 21,
        };

        self.read_pointer(index)
    }

    /// Disable the I/O permission bitmap for this task-state segment.
    #[inline]
    pub fn disable_io_map(&mut self) {
        let &mut Self(ref mut words) = self;

        let word = words.get_mut(25);

        match word {
            Some(word) => {
                let mut base = TaskIoMapBaseMut::wrap(word);

                base.const_merge(mem::size_of::<Self>() as u16);
            },
            None => unreachable!("the fixed TSS image always contains word twenty-five"),
        }
    }

    /// Returns the byte offset that disables the I/O permission bitmap.
    #[inline]
    #[must_use]
    pub fn io_map_base(&self) -> u16 {
        let &Self(ref words) = self;

        let word = words.get(25).copied();

        match word {
            Some(word) => TaskIoMapBase::wrap(&word).const_value(),
            None => unreachable!("the fixed TSS image always contains word twenty-five"),
        }
    }

    /// Writes one canonical pointer into a paired low and high word.
    fn write_pointer(&mut self, index: usize, pointer: La) {
        let &mut Self(ref mut words) = self;

        let value = pointer.bits();
        let low = U64Low32::wrap(&value).const_value();
        let high = U64High32::wrap(&value).const_value();
        let pair = words.get_mut(index..index + 2).and_then(<[u32]>::first_chunk_mut::<2>);

        match pair {
            Some(pair) => {
                pair[0] = low;
                pair[1] = high;
            },
            None => unreachable!("every task stack index names a complete word pair"),
        }
    }

    /// Reads one canonical pointer from a paired low and high word.
    fn read_pointer(&self, index: usize) -> La {
        let &Self(ref words) = self;

        let pair = words.get(index..index + 2).and_then(<[u32]>::first_chunk::<2>);
        let value = match pair {
            Some(pair) => {
                let mut value = u64::MIN;
                let mut low = U64Low32Mut::wrap(&mut value);

                low.const_merge(pair[0]);

                let mut high = U64High32Mut::wrap(&mut value);

                high.const_merge(pair[1]);

                value
            },
            None => unreachable!("every task stack index names a complete word pair"),
        };

        match La::new::<La48>(value) {
            Some(pointer) => pointer,
            None => match La::new::<La57>(value) {
                Some(pointer) => pointer,
                None => unreachable!("task stack mutation accepts only supported canonical linear addresses"),
            },
        }
    }
}

impl Default for TaskStateSegment {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use core::mem;

    use super::{InterruptStack, PrivilegeStack, TaskStateSegment};
    use crate::x86_64::paging::{La, La48, La57};

    #[test]
    fn task_state_layout_and_typed_stacks_round_trip() {
        let ring0 = La::new::<La48>(0x8000).expect("ring-zero stack must be canonical");
        let ist = La::new::<La48>(0x9000).expect("interrupt stack must be canonical");
        let mut state = TaskStateSegment::new();

        state.set_privilege_stack(PrivilegeStack::Ring0, ring0);
        state.set_interrupt_stack(InterruptStack::One, ist);

        assert_eq!(mem::size_of::<TaskStateSegment>(), 104);
        assert_eq!(mem::align_of::<TaskStateSegment>(), 4);
        assert_eq!(state.privilege_stack(PrivilegeStack::Ring0), ring0);
        assert_eq!(state.interrupt_stack(InterruptStack::One), ist);
        assert_eq!(state.io_map_base(), 104);
    }

    #[test]
    fn la57_only_stack_pointer_round_trips() {
        let pointer = La::new::<La57>(0x0001_0000_0000_0000).expect("test pointer must be LA57 canonical");
        assert!(!pointer.is_canonical::<La48>());

        let mut state = TaskStateSegment::new();

        state.set_privilege_stack(PrivilegeStack::Ring0, pointer);

        assert_eq!(state.privilege_stack(PrivilegeStack::Ring0), pointer);
    }
}
