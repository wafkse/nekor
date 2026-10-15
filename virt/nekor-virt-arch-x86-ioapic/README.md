# nekor-virt-arch-x86-ioapic

## Overview

x86 IOAPIC

## Responsibilities

Owns x86 IOAPIC-specific virtual device state and behavior.

## Architecture

Intended to model IOAPIC transitions independently of a particular host backend.

## Boundaries

It does not own the local APIC, generic peripheral contracts, or host interrupt routing.

## Status

The crate contains scaffolding only, IOAPIC functionality remains planned.
