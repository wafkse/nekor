/*
 * Layout queries, compile-time assertions, and compiler attributes shared by the KVM sources.
 */
#ifndef NEKOR_KVM_MISCENALLEOUS_H
#define NEKOR_KVM_MISCENALLEOUS_H

#include <stddef.h>

/*
 * Select the C or C++ syntax for alignment queries and compile-time assertions.
 */
#ifdef __cplusplus
#define NEKOR_KVM_ALIGNOF(target_type) alignof(target_type)
#define NEKOR_KVM_STATIC_ASSERT(target_condition, target_message) \
    static_assert(target_condition, target_message)
#else
#define NEKOR_KVM_ALIGNOF(target_type) _Alignof(target_type)
#define NEKOR_KVM_STATIC_ASSERT(target_condition, target_message) \
    _Static_assert(target_condition, target_message)
#endif

/*
 * Native size and member-offset queries used when validating Linux UAPI layouts.
 */
#define NEKOR_KVM_SIZEOF(target_type) sizeof(target_type)
#define NEKOR_KVM_OFFSETOF(target_type, target_member) offsetof(target_type, target_member)

/*
 * GCC and Clang attributes, with standard inline behavior and empty attribute fallbacks on
 * other compilers.
 */
#if defined(__GNUC__) || defined(__clang__)
#define NEKOR_KVM_ATTRIBUTE(...) __attribute__((__VA_ARGS__))
#define NEKOR_KVM_INLINE inline
#define NEKOR_KVM_FORCEINLINE inline NEKOR_KVM_ATTRIBUTE(always_inline)
#define NEKOR_KVM_NOINLINE NEKOR_KVM_ATTRIBUTE(noinline)
#define NEKOR_KVM_COLD NEKOR_KVM_ATTRIBUTE(cold)
#define NEKOR_KVM_HOT NEKOR_KVM_ATTRIBUTE(hot)
#define NEKOR_KVM_USED NEKOR_KVM_ATTRIBUTE(used)
#define NEKOR_KVM_UNUSED NEKOR_KVM_ATTRIBUTE(unused)
#define NEKOR_KVM_NORETURN NEKOR_KVM_ATTRIBUTE(noreturn)
#define NEKOR_KVM_WARN_UNUSED_RESULT NEKOR_KVM_ATTRIBUTE(warn_unused_result)
#define NEKOR_KVM_PACKED NEKOR_KVM_ATTRIBUTE(packed)
#define NEKOR_KVM_ALIGNED(target_alignment) NEKOR_KVM_ATTRIBUTE(aligned(target_alignment))
#define NEKOR_KVM_LIKELY(target_condition) __builtin_expect(!!(target_condition), 1)
#define NEKOR_KVM_UNLIKELY(target_condition) __builtin_expect(!!(target_condition), 0)
#else
#define NEKOR_KVM_ATTRIBUTE(...)
#define NEKOR_KVM_INLINE inline
#define NEKOR_KVM_FORCEINLINE inline
#define NEKOR_KVM_NOINLINE
#define NEKOR_KVM_COLD
#define NEKOR_KVM_HOT
#define NEKOR_KVM_USED
#define NEKOR_KVM_UNUSED
#define NEKOR_KVM_NORETURN
#define NEKOR_KVM_WARN_UNUSED_RESULT
#define NEKOR_KVM_PACKED
#define NEKOR_KVM_ALIGNED(target_alignment)
#define NEKOR_KVM_LIKELY(target_condition) (!!(target_condition))
#define NEKOR_KVM_UNLIKELY(target_condition) (!!(target_condition))
#endif

#endif /* NEKOR_KVM_MISCENALLEOUS_H */
