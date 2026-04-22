//! An umbrella module for fully qualified features.

use crate::feature::{Feature, Present, Signal};

#[allow(unused_imports, reason = "allow all output registers to be imported")]
use crate::arch::x86::{Cached, CpuidReg, Eax, Ebx, Ecx, Edx};

use nekor_bitwise::prelude::BitAt;

/// A macro to expand to a [`Feature`] implementor for a specific [`CpuId`]
/// invocation.
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
    /// A marker type to indicate support for the `XSAVE` instruction by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub XSave in Ecx for (0x01) use 26 (feature = "xsave")
);

feature!(
    /// A marker type to indicate support for the `XSAVEOPT` instruction by the processor.
    ///
    /// This has a dependency on the [`XSave`] feature.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub XSaveOpt in Eax for (0x0D, 0x01) use 0 (feature = "xsaveopt")
);

feature!(
    /// A marker type to indicate support for the `XSAVEC` instruction by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub XSaveC in Eax for (0x0D, 0x01) use 1 (feature = "xsavec")
);

feature!(
    /// A marker type to indicate support for the `XSAVES` instruction by the processor.
    ///
    /// This has a dependency on the [`XSave`] feature.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub XSaveS in Eax for (0x0D, 0x01) use 3 (feature = "xsaves")
);

feature!(
    /// A marker type to indicate support for the `CMPXCHG16B` instruction by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub CmpXChg16b in Ecx for (0x01) use 13 (feature = "cmpxchg16b")
);

feature!(
    /// A marker type to indicate support for the `MONITOR` and `MWAIT` instructions.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Monitor in Ecx for (0x01) use 3
);

feature!(
    /// A marker type to indicate support for the `MONITORX` and `MWAITX` instructions.
    ///
    /// This is only available on *AMD processors*.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub MonitorX in Ecx for (0x80000001) use 29
);

feature!(
    /// A marker type to indicate support for the instructions provided by the `WAITPKG` extension.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Waitpkg in Ecx for (0x07, 0x00) use 5
);

feature!(
    /// A marker type to indicate support for the `serialize` instruction.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Serialize in Edx for (0x07, 0x00) use 14
);

feature!(
    /// A marker type to indicate support for the `ADX` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Adx in Ebx for (0x07, 0x00) use 19 (feature = "adx")
);

feature!(
    /// A marker type to indicate support for the `AES` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Aes in Ecx for (0x01) use 25 (feature = "aes")
);

feature!(
    /// A marker type to indicate support for the `AVX` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx in Ecx for (0x01) use 28 (feature = "avx")
);

feature!(
    /// A marker type to indicate support for the `AVX2` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx2 in Ebx for (0x07, 0x00) use 5 (feature = "avx2")
);

feature!(
    /// A marker type to indicate support for the `AVX512BF16` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Bf16 in Eax for (0x07, 0x01) use 5 (feature = "avx512bf16")
);

feature!(
    /// A marker type to indicate support for the `AVX512BITALG` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Bitalg in Ecx for (0x07, 0x00) use 12 (feature = "avx512bitalg")
);

feature!(
    /// A marker type to indicate support for the `AVX512BW` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Bw in Ebx for (0x07, 0x00) use 30 (feature = "avx512bw")
);

feature!(
    /// A marker type to indicate support for the `AVX512CD` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Cd in Ebx for (0x07, 0x00) use 28 (feature = "avx512cd")
);

feature!(
    /// A marker type to indicate support for the `AVX512DQ` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Dq in Ebx for (0x07, 0x00) use 17 (feature = "avx512dq")
);

feature!(
    /// A marker type to indicate support for the `AVX512F` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512F in Ebx for (0x07, 0x00) use 16 (feature = "avx512f")
);

feature!(
    /// A marker type to indicate support for the `AVX512FP16` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Fp16 in Edx for (0x07, 0x00) use 23 (feature = "avx512fp16")
);

feature!(
    /// A marker type to indicate support for the `AVX512IFMA` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Ifma in Ebx for (0x07, 0x00) use 21 (feature = "avx512ifma")
);

feature!(
    /// A marker type to indicate support for the `AVX512VBMI` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Vbmi in Ecx for (0x07, 0x00) use 1 (feature = "avx512vbmi")
);

feature!(
    /// A marker type to indicate support for the `AVX512VBMI2` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Vbmi2 in Ecx for (0x07, 0x00) use 6 (feature = "avx512vbmi2")
);

feature!(
    /// A marker type to indicate support for the `AVX512VL` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Vl in Ebx for (0x07, 0x00) use 31 (feature = "avx512vl")
);

feature!(
    /// A marker type to indicate support for the `AVX512VNNI` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Vnni in Ecx for (0x07, 0x00) use 11 (feature = "avx512vnni")
);

feature!(
    /// A marker type to indicate support for the `AVX512VP2INTERSECT` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Vp2Intersect in Edx for (0x07, 0x00) use 8 (feature = "avx512vp2intersect")
);

feature!(
    /// A marker type to indicate support for the `AVX512VPOPCNTDQ` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Avx512Vpopcntdq in Ecx for (0x07, 0x00) use 14 (feature = "avx512vpopcntdq")
);

feature!(
    /// A marker type to indicate support for the `AVXIFMA` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub AvxIfma in Eax for (0x07, 0x01) use 23 (feature = "avxifma")
);

feature!(
    /// A marker type to indicate support for the `AVXNECONVERT` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub AvxNeConvert in Edx for (0x07, 0x01) use 5 (feature = "avxneconvert")
);

feature!(
    /// A marker type to indicate support for the `AVXVNNI` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub AvxVnni in Eax for (0x07, 0x01) use 4 (feature = "avxvnni")
);

feature!(
    /// A marker type to indicate support for the `AVXVNNIINT16` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub AvxVnniInt16 in Edx for (0x07, 0x01) use 10 (feature = "avxvnniint16")
);

feature!(
    /// A marker type to indicate support for the `AVXVNNIINT8` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub AvxVnniInt8 in Edx for (0x07, 0x01) use 4 (feature = "avxvnniint8")
);

feature!(
    /// A marker type to indicate support for the `BMI1` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Bmi1 in Ebx for (0x07, 0x00) use 3 (feature = "bmi1")
);

feature!(
    /// A marker type to indicate support for the `BMI2` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Bmi2 in Ebx for (0x07, 0x00) use 8 (feature = "bmi2")
);

feature!(
    /// A marker type to indicate support for the `F16C` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub F16c in Ecx for (0x01) use 29 (feature = "f16c")
);

feature!(
    /// A marker type to indicate support for the `FMA` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Fma in Ecx for (0x01) use 12 (feature = "fma")
);

feature!(
    /// A marker type to indicate support for the `FXSR` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Fxsr in Edx for (0x01) use 24 (feature = "fxsr")
);

feature!(
    /// A marker type to indicate support for the `GFNI` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Gfni in Ecx for (0x07, 0x00) use 8 (feature = "gfni")
);

feature!(
    /// A marker type to indicate support for the `KL` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Kl in Ecx for (0x07, 0x00) use 23 (feature = "kl")
);

feature!(
    /// A marker type to indicate support for the `LZCNT` instruction by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Lzcnt in Ecx for (0x80000001) use 5 (feature = "lzcnt")
);

feature!(
    /// A marker type to indicate support for the `MOVBE` instruction by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Movbe in Ecx for (0x01) use 22 (feature = "movbe")
);

feature!(
    /// A marker type to indicate support for the `PCLMULQDQ` instruction by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Pclmulqdq in Ecx for (0x01) use 1 (feature = "pclmulqdq")
);

feature!(
    /// A marker type to indicate support for the `POPCNT` instruction by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Popcnt in Ecx for (0x01) use 23 (feature = "popcnt")
);

feature!(
    /// A marker type to indicate support for the `RDRAND` instruction by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Rdrand in Ecx for (0x01) use 30 (feature = "rdrand")
);

feature!(
    /// A marker type to indicate support for the `RDSEED` instruction by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Rdseed in Ebx for (0x07, 0x00) use 18 (feature = "rdseed")
);

feature!(
    /// A marker type to indicate support for the `SHA` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sha in Ebx for (0x07, 0x00) use 29 (feature = "sha")
);

feature!(
    /// A marker type to indicate support for the `SHA512` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sha512 in Eax for (0x07, 0x01) use 0 (feature = "sha512")
);

feature!(
    /// A marker type to indicate support for the `SM3` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sm3 in Eax for (0x07, 0x01) use 1 (feature = "sm3")
);

feature!(
    /// A marker type to indicate support for the `SM4` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sm4 in Eax for (0x07, 0x01) use 2 (feature = "sm4")
);

feature!(
    /// A marker type to indicate support for the `SSE` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sse in Edx for (0x01) use 25 (feature = "sse")
);

feature!(
    /// A marker type to indicate support for the `SSE2` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sse2 in Edx for (0x01) use 26 (feature = "sse2")
);

feature!(
    /// A marker type to indicate support for the `SSE3` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sse3 in Ecx for (0x01) use 0 (feature = "sse3")
);

feature!(
    /// A marker type to indicate support for the `SSE4.1` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sse41 in Ecx for (0x01) use 19 (feature = "sse4.1")
);

feature!(
    /// A marker type to indicate support for the `SSE4.2` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sse42 in Ecx for (0x01) use 20 (feature = "sse4.2")
);

feature!(
    /// A marker type to indicate support for the `SSE4A` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Sse4a in Ecx for (0x80000001) use 6 (feature = "sse4a")
);

feature!(
    /// A marker type to indicate support for the `SSSE3` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Ssse3 in Ecx for (0x01) use 9 (feature = "ssse3")
);

feature!(
    /// A marker type to indicate support for the `TBM` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Tbm in Ecx for (0x80000001) use 21 (feature = "tbm")
);

feature!(
    /// A marker type to indicate support for the `VAES` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Vaes in Ecx for (0x07, 0x00) use 9 (feature = "vaes")
);

feature!(
    /// A marker type to indicate support for the `VPCLMULQDQ` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Vpclmulqdq in Ecx for (0x07, 0x00) use 10 (feature = "vpclmulqdq")
);

feature!(
    /// A marker type to indicate support for the `WIDEKL` architectural extension by the processor.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
    pub Widekl in Ecx for (0x07, 0x00) use 25 (feature = "widekl")
);
