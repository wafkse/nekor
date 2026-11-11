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
- Architecture-independent parsing and source-defined domain validation.
- Precise source diagnostics, without requiring a general-purpose language runtime.

Verte describes data and its constraints. Configuration authors specify values and validity requirements independently of how applications use the resulting data. Configuration fields are independently evaluated and cannot reference one another. Constants provide controlled reuse without introducing dependencies between configuration values.

## Language

Verte draws inspiration from TOML's hierarchical representation, Devicetree's hardware-oriented data model, and Figment's configuration composition semantics.

A representative configuration may look like:

```verte
inherit(merge)

domain physical: addr = { width = 52 }
domain virtual: addr = { width = 64 }

const page_size: size = 4_KiB
const stack_pages: u16 = 32

rule page_bound = 4_KiB..=1_GiB & ^2
rule page_aligned = % page_size

type page = size @ page_bound
type page_address = addr[physical] @ page_aligned

[arch]
page_size: page = page_size
stack_size: size = page_size * stack_pages

[layout]
physical_base: page_address = 0x1000_0000
physical_memory: range[addr[physical]] = 0x1000_0000..+2_GiB
virtual_memory: range[addr[virtual]] = 0xffff_8000_0000_0000..+2_GiB

[serial]
frequency: frequency = 48_MHz
baud: symbolrate = 115_200_Bd

[drivers]
admerge(enabled: [string] = ["uart", "virtio"])
```

Address spaces are nominal source-declared domains. Physical, virtual, and DMA spaces can remain distinct even when their coordinates have the same width. Dimensional quantities likewise preserve physical dimensions and semantic kinds during arithmetic.

Types may be associated with reusable constraints that express validity requirements directly in the data model.

Configuration documents may be composed using `merge`, `join`, `admerge`, or `adjoin` strategies. Inheritance may also be prohibited entirely. Composition is subject to type compatibility and inherited validation requirements.

## Vertel

*Vertel* is the proposed expression language built upon Verte's data model and type system.

Verte establishes the representation and validation of typed data, while Vertel is intended to extend its expression capabilities independently of configuration-specific semantics.

Both languages should share a consistent understanding of types, values, dimensional arithmetic, and ranges.

## Implementation

Verte is written in Rust and uses [Aldebaran](https://github.com/wafkse/aldebaran) for lexical analysis, parsing infrastructure, source tracking, and diagnostics.

Language-specific responsibilities, including type checking, dimensional analysis, constant evaluation, validation, and configuration composition, remain within Verte.

## Status

Parsing, represented scalar arithmetic, structural type admission, and typed composition are implemented as library components. Rule evaluation, declaration resolution, and host source loading remain unfinished, so the README example cannot yet be evaluated into a configuration by a single library entry point.
