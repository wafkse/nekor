# eustace-kvm

## Overview

Safe Linux KVM backend.

## Responsibilities

Owns KVM descriptors, guest RAM mappings, vCPU execution, and exit handling.

## Architecture

Start from `prelude`, open `resource::state::Kvm`, then use `Kvm::machine` to create a machine.

## Boundaries

Kernel declarations and host operations belong to
[`eustace-kvm-sys`](../eustace-kvm-sys/README.md). Shared address, memory, I/O, and MMIO types
belong to their domain crates.

## Status

The default `stable` feature selects the stable kernel pin. Use `--no-default-features --features lts`
to select the LTS pin.
