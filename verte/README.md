# Verte

Verte is the proposed future configuration language for Nekor, and the data definition language baseline for *Vertel*, the expression language.

## Goals

Verte aims to provide:

- A small, statically typed, declarative language with an unambiguous grammar.
- Explicit types for all configuration values, including user-defined and constrained types.
- First-class dimensional quantities, units, addresses, ranges, and hardware resources.
- Reusable, composable validation rules expressed through a concise symbolic syntax.
- Immutable constants and deterministic expression evaluation.
- Predictable configuration inheritance, with configurable merge strategies and per-field overrides.
- Architecture-independent parsing and target-aware validation.
- Precise source diagnostics, without requiring a general-purpose language runtime.

Verte is intended to describe data and its constraints, rather than prescribe how that data is computed or consumed. Configuration fields are independently evaluated and cannot reference one another. Constants provide controlled reuse without introducing dependencies between configuration values.

## Language

Verte draws inspiration from TOML's hierarchical representation, Devicetree's hardware-oriented data model, and Figment's configuration composition semantics.

A representative configuration may look like:

```verte
inherit merge

const page-size: size = 4_KiB
const stack-pages: u16 = 32

rule page-bound = 4_KiB..=1_GiB & ^2
rule page-aligned = % page-size

type page = size @ page-bound
type page-address = physical-address @ page-aligned

[arch]
page-size: page = page-size
stack-size: size = page-size * stack-pages

[layout]
physical-base: page-address = 0x1000_000
physical-memory: range[physical-address] = 0x1000_000..+2_GiB
virtual-memory: range[virtual-address] = 0xffff_8000_0000_0000..+2_GiB

[serial]
frequency: frequency = 48_MHz
baud: symbolrate = 115_200_Bd

[drivers]
admerge enabled: [string] = ["uart", "virtio"]
```

The language distinguishes physical, virtual, and logical addresses, and provides dimensional analysis for physical quantities. Operations involving incompatible dimensions or address spaces are rejected during validation.

Types may be associated with reusable constraints, allowing validity requirements to be expressed directly in the data model rather than through separate assertions.

Configuration documents may be composed using `merge`, `join`, `admerge`, or `adjoin` strategies. Inheritance may also be prohibited entirely. Composition is subject to type compatibility and inherited validation requirements.

## Vertel

*Vertel* is the proposed expression language built upon Verte's data model and type system.

Verte establishes the representation and validation of typed data, while Vertel is intended to extend its expression capabilities independently of configuration-specific semantics.

Both languages should share a consistent understanding of types, values, dimensional arithmetic, and ranges.

## Implementation

Verte is written in Rust and uses [Aldebaran](https://github.com/wafkse/aldebaran) for lexical analysis, parsing infrastructure, source tracking, and diagnostics.

Language-specific responsibilities, including type checking, dimensional analysis, constant evaluation, validation, and configuration composition, remain within Verte.

The implementation is intended to remain modular, deterministic, and suitable for environments where dependency size and auditability matter.

## Status

Verte is currently in the early design and implementation stage. The grammar, type system, and language semantics are subject to change.

The initial objective is to establish a consistent data definition language and typed intermediate representation before extending the expression model through Vertel.
