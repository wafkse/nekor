# nekor-virt-kvm-sys

## Overview

Linux KVM system interface

## Responsibilities

Provides low-level system interface bindings or representations for the Linux KVM backend.

## Architecture

Intended to isolate KVM-specific operating-system interface details from shared virtualization contracts.

## Boundaries

It does not define backend-neutral architecture or guest ABI semantics.

## Status

The crate contains scaffolding only, system bindings remain planned.
