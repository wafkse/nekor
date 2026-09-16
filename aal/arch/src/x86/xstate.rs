//! x86 extended-control and XSTATE component masks.
//!
//! XCR0 is represented as its complete architectural bitmap. Named bit views
//! expose known user-state components without turning the register into a bag
//! of integer masks. Unknown component bits remain representable for transport.

use nekor_bitwise::prelude::{Bit, BitAt, Counterpart, Field};

/// Proof that the current processor exposes the XFD architectural MSRs and semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Xfd(());

impl Xfd {
    /// Create the XFD capability proof for the current processor.
    ///
    /// # Safety
    ///
    /// The current processor must report the XFD architectural feature.
    #[inline]
    #[must_use]
    pub const unsafe fn assume() -> Self {
        Self(())
    }
}

/// Legacy x87 state enablement in [`Xcr0`].
pub type Xcr0X87<'value> = Bit<'value, u64, 0>;

/// SSE state enablement in [`Xcr0`].
pub type Xcr0Sse<'value> = Bit<'value, u64, 1>;

/// AVX state enablement in [`Xcr0`].
pub type Xcr0Avx<'value> = Bit<'value, u64, 2>;

/// MPX bound-register state enablement in [`Xcr0`].
pub type Xcr0BndRegs<'value> = Bit<'value, u64, 3>;

/// MPX bound-configuration state enablement in [`Xcr0`].
pub type Xcr0BndCsr<'value> = Bit<'value, u64, 4>;

/// AVX-512 opmask state enablement in [`Xcr0`].
pub type Xcr0Opmask<'value> = Bit<'value, u64, 5>;

/// AVX-512 upper ZMM state enablement in [`Xcr0`].
pub type Xcr0ZmmHi256<'value> = Bit<'value, u64, 6>;

/// AVX-512 high ZMM register state enablement in [`Xcr0`].
pub type Xcr0Hi16Zmm<'value> = Bit<'value, u64, 7>;

/// Reserved XCR0 bit eight.
pub type Xcr0Reserved8<'value> = Bit<'value, u64, 8>;

/// Protection-key state enablement in [`Xcr0`].
pub type Xcr0Pkru<'value> = Bit<'value, u64, 9>;

/// Reserved XCR0 bits ten through 16.
pub type Xcr0Reserved10_16<'value> = Field<'value, 10, 16, u64>;

/// AMX tile-configuration state enablement in [`Xcr0`].
pub type Xcr0TileConfig<'value> = Bit<'value, u64, 17>;

/// AMX tile-data state enablement in [`Xcr0`].
pub type Xcr0TileData<'value> = Bit<'value, u64, 18>;

/// Intel APX extended general-purpose register state enablement in [`Xcr0`].
pub type Xcr0Apx<'value> = Bit<'value, u64, 19>;

/// Reserved XCR0 bits 20 through 63.
pub type Xcr0Reserved20_63<'value> = Field<'value, 20, 63, u64>;

/// Mutable legacy x87 state enablement.
pub type Xcr0X87Mut<'value> = <Xcr0X87<'value> as Counterpart>::Mut;

/// Mutable SSE state enablement.
pub type Xcr0SseMut<'value> = <Xcr0Sse<'value> as Counterpart>::Mut;

/// Mutable AVX state enablement.
pub type Xcr0AvxMut<'value> = <Xcr0Avx<'value> as Counterpart>::Mut;

/// Mutable MPX bound-register state enablement.
pub type Xcr0BndRegsMut<'value> = <Xcr0BndRegs<'value> as Counterpart>::Mut;

/// Mutable MPX bound-configuration state enablement.
pub type Xcr0BndCsrMut<'value> = <Xcr0BndCsr<'value> as Counterpart>::Mut;

/// Mutable AVX-512 opmask state enablement.
pub type Xcr0OpmaskMut<'value> = <Xcr0Opmask<'value> as Counterpart>::Mut;

/// Mutable AVX-512 upper ZMM state enablement.
pub type Xcr0ZmmHi256Mut<'value> = <Xcr0ZmmHi256<'value> as Counterpart>::Mut;

/// Mutable AVX-512 high ZMM register state enablement.
pub type Xcr0Hi16ZmmMut<'value> = <Xcr0Hi16Zmm<'value> as Counterpart>::Mut;

/// Mutable protection-key state enablement.
pub type Xcr0PkruMut<'value> = <Xcr0Pkru<'value> as Counterpart>::Mut;

/// Mutable AMX tile-configuration state enablement.
pub type Xcr0TileConfigMut<'value> = <Xcr0TileConfig<'value> as Counterpart>::Mut;

/// Mutable AMX tile-data state enablement.
pub type Xcr0TileDataMut<'value> = <Xcr0TileData<'value> as Counterpart>::Mut;

/// Mutable Intel APX state enablement.
pub type Xcr0ApxMut<'value> = <Xcr0Apx<'value> as Counterpart>::Mut;

/// Complete XCR0 component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves every XCR0 component bit. Safe
// field mutation changes only the selected component while unknown bits remain intact.
pub struct Xcr0(u64);

impl Xcr0 {
    /// Empty component bitmap.
    pub const EMPTY: Self = Self(0);

    /// Preserve one complete XCR0 component bitmap.
    #[inline]
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the complete component bitmap.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> u64 {
        let Self(bits) = self;

        bits
    }

    /// Return whether every required component is present.
    #[inline]
    #[must_use]
    pub const fn contains(self, required: Self) -> bool {
        let Self(bits) = self;

        let Self(required) = required;

        bits & required == required
    }

    /// Combine every component present in either bitmap.
    #[inline]
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        let Self(bits) = self;

        let Self(other) = other;

        Self(bits | other)
    }

    /// Retain only components present in both bitmaps.
    #[inline]
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        let Self(bits) = self;

        let Self(other) = other;

        Self(bits & other)
    }

    /// Return whether no component is selected.
    #[inline]
    #[must_use]
    pub const fn is_empty(self) -> bool {
        let Self(bits) = self;

        bits == 0
    }

    /// Borrow one component bit selected by its architectural index.
    #[inline]
    #[must_use]
    pub const fn component<const N: usize>(&self) -> Bit<'_, u64, N>
    where
        u64: BitAt<N>,
    {
        let &Self(ref bits) = self;

        Bit::wrap(bits)
    }

    /// Mutably borrow one component bit selected by its architectural index.
    #[inline]
    pub const fn component_mut<const N: usize>(&mut self) -> <Bit<'_, u64, N> as Counterpart>::Mut
    where
        u64: BitAt<N>,
    {
        let &mut Self(ref mut bits) = self;

        <Bit<'_, u64, N> as Counterpart>::Mut::wrap(bits)
    }

    /// Borrow legacy x87 state enablement.
    #[inline]
    #[must_use]
    pub const fn x87(&self) -> Xcr0X87<'_> {
        let &Self(ref bits) = self;

        Xcr0X87::wrap(bits)
    }

    /// Mutably borrow legacy x87 state enablement.
    #[inline]
    pub const fn x87_mut(&mut self) -> Xcr0X87Mut<'_> {
        let &mut Self(ref mut bits) = self;

        Xcr0X87Mut::wrap(bits)
    }

    /// Borrow SSE state enablement.
    #[inline]
    #[must_use]
    pub const fn sse(&self) -> Xcr0Sse<'_> {
        let &Self(ref bits) = self;

        Xcr0Sse::wrap(bits)
    }

    /// Mutably borrow SSE state enablement.
    #[inline]
    pub const fn sse_mut(&mut self) -> Xcr0SseMut<'_> {
        let &mut Self(ref mut bits) = self;

        Xcr0SseMut::wrap(bits)
    }

    /// Borrow AVX state enablement.
    #[inline]
    #[must_use]
    pub const fn avx(&self) -> Xcr0Avx<'_> {
        let &Self(ref bits) = self;

        Xcr0Avx::wrap(bits)
    }

    /// Mutably borrow AVX state enablement.
    #[inline]
    pub const fn avx_mut(&mut self) -> Xcr0AvxMut<'_> {
        let &mut Self(ref mut bits) = self;

        Xcr0AvxMut::wrap(bits)
    }

    /// Borrow MPX bound-register state enablement.
    #[inline]
    #[must_use]
    pub const fn bndregs(&self) -> Xcr0BndRegs<'_> {
        let &Self(ref bits) = self;

        Xcr0BndRegs::wrap(bits)
    }

    /// Mutably borrow MPX bound-register state enablement.
    #[inline]
    pub const fn bndregs_mut(&mut self) -> Xcr0BndRegsMut<'_> {
        let &mut Self(ref mut bits) = self;

        Xcr0BndRegsMut::wrap(bits)
    }

    /// Borrow MPX bound-configuration state enablement.
    #[inline]
    #[must_use]
    pub const fn bndcsr(&self) -> Xcr0BndCsr<'_> {
        let &Self(ref bits) = self;

        Xcr0BndCsr::wrap(bits)
    }

    /// Mutably borrow MPX bound-configuration state enablement.
    #[inline]
    pub const fn bndcsr_mut(&mut self) -> Xcr0BndCsrMut<'_> {
        let &mut Self(ref mut bits) = self;

        Xcr0BndCsrMut::wrap(bits)
    }

    /// Borrow AVX-512 opmask state enablement.
    #[inline]
    #[must_use]
    pub const fn opmask(&self) -> Xcr0Opmask<'_> {
        let &Self(ref bits) = self;

        Xcr0Opmask::wrap(bits)
    }

    /// Mutably borrow AVX-512 opmask state enablement.
    #[inline]
    pub const fn opmask_mut(&mut self) -> Xcr0OpmaskMut<'_> {
        let &mut Self(ref mut bits) = self;

        Xcr0OpmaskMut::wrap(bits)
    }

    /// Borrow AVX-512 upper ZMM state enablement.
    #[inline]
    #[must_use]
    pub const fn zmm_hi256(&self) -> Xcr0ZmmHi256<'_> {
        let &Self(ref bits) = self;

        Xcr0ZmmHi256::wrap(bits)
    }

    /// Mutably borrow AVX-512 upper ZMM state enablement.
    #[inline]
    pub const fn zmm_hi256_mut(&mut self) -> Xcr0ZmmHi256Mut<'_> {
        let &mut Self(ref mut bits) = self;

        Xcr0ZmmHi256Mut::wrap(bits)
    }

    /// Borrow AVX-512 high ZMM register state enablement.
    #[inline]
    #[must_use]
    pub const fn hi16_zmm(&self) -> Xcr0Hi16Zmm<'_> {
        let &Self(ref bits) = self;

        Xcr0Hi16Zmm::wrap(bits)
    }

    /// Mutably borrow AVX-512 high ZMM register state enablement.
    #[inline]
    pub const fn hi16_zmm_mut(&mut self) -> Xcr0Hi16ZmmMut<'_> {
        let &mut Self(ref mut bits) = self;

        Xcr0Hi16ZmmMut::wrap(bits)
    }

    /// Borrow reserved XCR0 bit eight.
    #[inline]
    #[must_use]
    pub const fn reserved_8(&self) -> Xcr0Reserved8<'_> {
        let &Self(ref bits) = self;

        Xcr0Reserved8::wrap(bits)
    }

    /// Borrow protection-key state enablement.
    #[inline]
    #[must_use]
    pub const fn pkru(&self) -> Xcr0Pkru<'_> {
        let &Self(ref bits) = self;

        Xcr0Pkru::wrap(bits)
    }

    /// Mutably borrow protection-key state enablement.
    #[inline]
    pub const fn pkru_mut(&mut self) -> Xcr0PkruMut<'_> {
        let &mut Self(ref mut bits) = self;

        Xcr0PkruMut::wrap(bits)
    }

    /// Borrow reserved XCR0 bits ten through 16.
    #[inline]
    #[must_use]
    pub const fn reserved_10_16(&self) -> Xcr0Reserved10_16<'_> {
        let &Self(ref bits) = self;

        Xcr0Reserved10_16::wrap(bits)
    }

    /// Borrow AMX tile-configuration state enablement.
    #[inline]
    #[must_use]
    pub const fn tile_config(&self) -> Xcr0TileConfig<'_> {
        let &Self(ref bits) = self;

        Xcr0TileConfig::wrap(bits)
    }

    /// Mutably borrow AMX tile-configuration state enablement.
    #[inline]
    pub const fn tile_config_mut(&mut self) -> Xcr0TileConfigMut<'_> {
        let &mut Self(ref mut bits) = self;

        Xcr0TileConfigMut::wrap(bits)
    }

    /// Borrow AMX tile-data state enablement.
    #[inline]
    #[must_use]
    pub const fn tile_data(&self) -> Xcr0TileData<'_> {
        let &Self(ref bits) = self;

        Xcr0TileData::wrap(bits)
    }

    /// Mutably borrow AMX tile-data state enablement.
    #[inline]
    pub const fn tile_data_mut(&mut self) -> Xcr0TileDataMut<'_> {
        let &mut Self(ref mut bits) = self;

        Xcr0TileDataMut::wrap(bits)
    }

    /// Borrow Intel APX state enablement.
    #[inline]
    #[must_use]
    pub const fn apx(&self) -> Xcr0Apx<'_> {
        let &Self(ref bits) = self;

        Xcr0Apx::wrap(bits)
    }

    /// Mutably borrow Intel APX state enablement.
    #[inline]
    pub const fn apx_mut(&mut self) -> Xcr0ApxMut<'_> {
        let &mut Self(ref mut bits) = self;

        Xcr0ApxMut::wrap(bits)
    }

    /// Borrow reserved XCR0 bits 20 through 63.
    #[inline]
    #[must_use]
    pub const fn reserved_20_63(&self) -> Xcr0Reserved20_63<'_> {
        let &Self(ref bits) = self;

        Xcr0Reserved20_63::wrap(bits)
    }
}

#[cfg(test)]
mod tests {
    use nekor_bitwise::prelude::State;

    use super::Xcr0;

    #[test]
    fn named_component_views_preserve_independent_state() {
        let mut value = Xcr0::EMPTY;

        value.x87_mut().const_set(State::Set);
        value.sse_mut().const_set(State::Set);
        value.avx_mut().const_set(State::Set);

        assert_eq!(value.x87().state(), State::Set);
        assert_eq!(value.sse().state(), State::Set);
        assert_eq!(value.avx().state(), State::Set);
        assert_eq!(value.pkru().state(), State::Cleared);
    }

    #[test]
    fn component_containment_uses_typed_register_identity() {
        let mut required = Xcr0::EMPTY;
        let mut provided = Xcr0::EMPTY;

        required.x87_mut().const_set(State::Set);
        required.sse_mut().const_set(State::Set);

        provided.x87_mut().const_set(State::Set);
        provided.sse_mut().const_set(State::Set);
        provided.avx_mut().const_set(State::Set);

        assert!(provided.contains(required));
        assert!(!required.contains(provided));
    }
}
