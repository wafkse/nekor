# eustace-arch-x86

## Overview

x86 architecture components

## Responsibilities

Defines shared x86 architectural values and peripheral modules.

## Architecture

The `eustace-arch` crate exposes these definitions through its `x86` module.

The `apic`, `hpet`, `ioapic`, and `pic` modules belong to this crate.

## Boundaries

Hypervisor resource ownership and backend-specific execution remain separate responsibilities.

## Status

Provides shared x86 values. The peripheral modules contain scaffolding only,
their device behavior remains planned.
