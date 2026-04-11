//! The *Global Descriptor Table*.

use nekor_aal_agnostic::partitioned::Partitioned;

pub struct GlobalDescriptorTable<const N: usize>([RawSegmentDescriptor; N]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawAccessByte(u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(
    C,
    // NOTE(alignment): This requires 8-byte alignment.
    align(8)
)]
pub struct RawSegmentDescriptor {
    /// The bits `0..16` of the *Limit* field of this *Segment Descriptor*.
    limit_0_16: Partitioned<0, 15, u32>,

    /// The bits `0..16` of the *Base* field of this *Segment Descriptor*.
    base_0_16: Partitioned<0, 15, u64>,

    /// The bits `16..24` of the *Base* field of this *Segment Descriptor*.
    base_16_24: Partitioned<16, 23, u64>,

    /// The *Access Byte* field of this *Segment Descriptor*.
    access_byte: RawAccessByte,

    /// The bits `16..20` of the *Limit* field of this *Segment Descriptor*.
    limit_16_20: Partitioned<16, 19, u32>,
}
