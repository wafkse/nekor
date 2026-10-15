# nekor-virt-arch-x86-apic

## Overview

x86 APIC

## Responsibilities

Owns x86 APIC-specific virtual device state and behavior.

## Architecture

Intended to model APIC transitions independently from host backend execution.

## Boundaries

It does not own generic peripheral protocols or a KVM/WHP implementation.

## Status

The crate contains scaffolding only, APIC functionality remains planned.
