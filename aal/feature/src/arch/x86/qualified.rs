//! Named instruction-support queries.
//!
//! Queries use configured target features when present and otherwise inspect cached
//! CPUID results. A reported instruction still needs its operating-system controls
//! and privilege requirements established before use.

use nekor_bitwise::prelude::BitAt;

use crate::{
    arch::x86::{Cached, CpuidReg, Eax, Ebx, Ecx, Edx},
    feature::{Feature, Present, Signal},
};

/// Define a named CPUID bit query with an optional compile-time baseline.
///
/// Optionally takes a `rustc`-recognized target feature for the `x86{,_64}` architecture.
macro_rules! feature {
    () => {};
    (
        $(
            #[$target_meta:meta]
        )*

        $target_vis:vis $target_name:ident in $target_register:ident for ($target_leaf:literal $(, $target_subleaf:literal)? ) use $target_bit:literal $((feature = $target_feature:literal))?
    ) => {
        tokel::stream!(
            $(
                #[$target_meta]
            )*
            $target_vis enum $target_name {}

            impl Feature for $target_name {
                // TODO: Add auto-generated init_array checks for baselined features.
                //
                // One liner to generate cargo features:
                //
                // grep -P "(pub)?\s+(\K[\w_][\w_0-9]*)\s+(\w{3})\s+for\s+\(0[xX]([a-fA-F0-9]+),\s+0[xX]([a-fA-F0-9]+)\)\s+use\s+(\d+)" aal/feature/src/arch/x86/qualified.rs | awk '{ print $2 }' | tr '[:upper:]' '[:lower:]' | xargs -I{} printf "feature-baseline-%s = []\n" "{}"
                //
                // Some features do not have directly rustc-recognised names, move those to cargo features.

                $(
                    #[cfg(target_feature = $target_feature)]
                    const PREDETERMINED: Option<Signal> = Some(Signal::Guaranteed(Present::Yes));
                )?

                #[inline(always)]
                fn supported() -> Signal {
                    Self::PREDETERMINED
                        .unwrap_or_else(
                            || {
                                let CpuidReg(ref target_state, ..) = CpuidReg::<$target_register, { $target_leaf } $(, $target_subleaf)?>::lazy();

                                Signal::state(
                                    BitAt::<$target_bit>::get(target_state)
                                )
                            }
                        )
                }
            }
        );
    };
}

feature!(
    /// Instruction support for the `XSAVE` instruction.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub XSave in Ecx for (0x01) use 26 (feature = "xsave")
);

feature!(
    /// Instruction support for the `XSAVEOPT` instruction.
    ///
    /// This has a dependency on the [`XSave`] feature.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub XSaveOpt in Eax for (0x0D, 0x01) use 0 (feature = "xsaveopt")
);

feature!(
    /// Instruction support for the `XSAVEC` instruction.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub XSaveC in Eax for (0x0D, 0x01) use 1 (feature = "xsavec")
);

feature!(
    /// Instruction support for the `XSAVES` instruction.
    ///
    /// This has a dependency on the [`XSave`] feature.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub XSaveS in Eax for (0x0D, 0x01) use 3 (feature = "xsaves")
);

feature!(
    /// Instruction support for the `CMPXCHG16B` instruction.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub CmpXChg16b in Ecx for (0x01) use 13 (feature = "cmpxchg16b")
);

feature!(
    /// Instruction support for the `MONITOR` and `MWAIT` instructions.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Monitor in Ecx for (0x01) use 3
);

feature!(
    /// Instruction support for the `MONITORX` and `MWAITX` instructions.
    ///
    /// This is only available on *AMD processors*.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub MonitorX in Ecx for (0x8000_0001) use 29
);

feature!(
    /// Instruction support for the instructions provided by the `WAITPKG` extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Waitpkg in Ecx for (0x07, 0x00) use 5
);

feature!(
    /// Instruction support for the `serialize` instruction.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Serialize in Edx for (0x07, 0x00) use 14
);

feature!(
    /// Instruction support for the `ADX` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Adx in Ebx for (0x07, 0x00) use 19 (feature = "adx")
);

feature!(
    /// Instruction support for the `AES` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Aes in Ecx for (0x01) use 25 (feature = "aes")
);

feature!(
    /// Instruction support for the `AVX` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx in Ecx for (0x01) use 28 (feature = "avx")
);

feature!(
    /// Instruction support for the `AVX2` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx2 in Ebx for (0x07, 0x00) use 5 (feature = "avx2")
);

feature!(
    /// Instruction support for the `AVX512BF16` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Bf16 in Eax for (0x07, 0x01) use 5 (feature = "avx512bf16")
);

feature!(
    /// Instruction support for the `AVX512BITALG` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Bitalg in Ecx for (0x07, 0x00) use 12 (feature = "avx512bitalg")
);

feature!(
    /// Instruction support for the `AVX512BW` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Bw in Ebx for (0x07, 0x00) use 30 (feature = "avx512bw")
);

feature!(
    /// Instruction support for the `AVX512CD` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Cd in Ebx for (0x07, 0x00) use 28 (feature = "avx512cd")
);

feature!(
    /// Instruction support for the `AVX512DQ` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Dq in Ebx for (0x07, 0x00) use 17 (feature = "avx512dq")
);

feature!(
    /// Instruction support for the `AVX512F` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512F in Ebx for (0x07, 0x00) use 16 (feature = "avx512f")
);

feature!(
    /// Instruction support for the `AVX512FP16` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Fp16 in Edx for (0x07, 0x00) use 23 (feature = "avx512fp16")
);

feature!(
    /// Instruction support for the `AVX512IFMA` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Ifma in Ebx for (0x07, 0x00) use 21 (feature = "avx512ifma")
);

feature!(
    /// Instruction support for the `AVX512VBMI` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Vbmi in Ecx for (0x07, 0x00) use 1 (feature = "avx512vbmi")
);

feature!(
    /// Instruction support for the `AVX512VBMI2` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Vbmi2 in Ecx for (0x07, 0x00) use 6 (feature = "avx512vbmi2")
);

feature!(
    /// Instruction support for the `AVX512VL` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Vl in Ebx for (0x07, 0x00) use 31 (feature = "avx512vl")
);

feature!(
    /// Instruction support for the `AVX512VNNI` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Vnni in Ecx for (0x07, 0x00) use 11 (feature = "avx512vnni")
);

feature!(
    /// Instruction support for the `AVX512VP2INTERSECT` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Vp2Intersect in Edx for (0x07, 0x00) use 8 (feature = "avx512vp2intersect")
);

feature!(
    /// Instruction support for the `AVX512VPOPCNTDQ` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Vpopcntdq in Ecx for (0x07, 0x00) use 14 (feature = "avx512vpopcntdq")
);

feature!(
    /// Instruction support for the `AVXIFMA` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub AvxIfma in Eax for (0x07, 0x01) use 23 (feature = "avxifma")
);

feature!(
    /// Instruction support for the `AVXNECONVERT` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub AvxNeConvert in Edx for (0x07, 0x01) use 5 (feature = "avxneconvert")
);

feature!(
    /// Instruction support for the `AVXVNNI` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub AvxVnni in Eax for (0x07, 0x01) use 4 (feature = "avxvnni")
);

feature!(
    /// Instruction support for the `AVXVNNIINT16` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub AvxVnniInt16 in Edx for (0x07, 0x01) use 10 (feature = "avxvnniint16")
);

feature!(
    /// Instruction support for the `AVXVNNIINT8` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub AvxVnniInt8 in Edx for (0x07, 0x01) use 4 (feature = "avxvnniint8")
);

feature!(
    /// Instruction support for the `BMI1` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Bmi1 in Ebx for (0x07, 0x00) use 3 (feature = "bmi1")
);

feature!(
    /// Instruction support for the `BMI2` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Bmi2 in Ebx for (0x07, 0x00) use 8 (feature = "bmi2")
);

feature!(
    /// Instruction support for the `F16C` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub F16c in Ecx for (0x01) use 29 (feature = "f16c")
);

feature!(
    /// Instruction support for the `FMA` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Fma in Ecx for (0x01) use 12 (feature = "fma")
);

feature!(
    /// Instruction support for the `FXSR` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Fxsr in Edx for (0x01) use 24 (feature = "fxsr")
);

feature!(
    /// Instruction support for the `GFNI` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Gfni in Ecx for (0x07, 0x00) use 8 (feature = "gfni")
);

feature!(
    /// Instruction support for the `KL` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Kl in Ecx for (0x07, 0x00) use 23 (feature = "kl")
);

feature!(
    /// Instruction support for the `LZCNT` instruction.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Lzcnt in Ecx for (0x8000_0001) use 5 (feature = "lzcnt")
);

feature!(
    /// Instruction support for the `MOVBE` instruction.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Movbe in Ecx for (0x01) use 22 (feature = "movbe")
);

feature!(
    /// Instruction support for the `PCLMULQDQ` instruction.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Pclmulqdq in Ecx for (0x01) use 1 (feature = "pclmulqdq")
);

feature!(
    /// Instruction support for the `POPCNT` instruction.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Popcnt in Ecx for (0x01) use 23 (feature = "popcnt")
);

feature!(
    /// Instruction support for the `RDRAND` instruction.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Rdrand in Ecx for (0x01) use 30 (feature = "rdrand")
);

feature!(
    /// Instruction support for the `RDSEED` instruction.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Rdseed in Ebx for (0x07, 0x00) use 18 (feature = "rdseed")
);

feature!(
    /// Instruction support for the `SHA` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sha in Ebx for (0x07, 0x00) use 29 (feature = "sha")
);

feature!(
    /// Instruction support for the `SHA512` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sha512 in Eax for (0x07, 0x01) use 0 (feature = "sha512")
);

feature!(
    /// Instruction support for the `SM3` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sm3 in Eax for (0x07, 0x01) use 1 (feature = "sm3")
);

feature!(
    /// Instruction support for the `SM4` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sm4 in Eax for (0x07, 0x01) use 2 (feature = "sm4")
);

feature!(
    /// Instruction support for the `SSE` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sse in Edx for (0x01) use 25 (feature = "sse")
);

feature!(
    /// Instruction support for the `SSE2` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sse2 in Edx for (0x01) use 26 (feature = "sse2")
);

feature!(
    /// Instruction support for the `SSE3` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sse3 in Ecx for (0x01) use 0 (feature = "sse3")
);

feature!(
    /// Instruction support for the `SSE4.1` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sse41 in Ecx for (0x01) use 19 (feature = "sse4.1")
);

feature!(
    /// Instruction support for the `SSE4.2` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sse42 in Ecx for (0x01) use 20 (feature = "sse4.2")
);

feature!(
    /// Instruction support for the `SSE4A` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sse4a in Ecx for (0x8000_0001) use 6 (feature = "sse4a")
);

feature!(
    /// Instruction support for the `SSSE3` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Ssse3 in Ecx for (0x01) use 9 (feature = "ssse3")
);

feature!(
    /// Instruction support for the `TBM` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Tbm in Ecx for (0x8000_0001) use 21 (feature = "tbm")
);

feature!(
    /// Instruction support for the `VAES` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Vaes in Ecx for (0x07, 0x00) use 9 (feature = "vaes")
);

feature!(
    /// Instruction support for the `VPCLMULQDQ` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Vpclmulqdq in Ecx for (0x07, 0x00) use 10 (feature = "vpclmulqdq")
);

feature!(
    /// Instruction support for the `WIDEKL` architectural extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Widekl in Ecx for (0x07, 0x00) use 25 (feature = "widekl")
);
