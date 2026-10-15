# nekor-virt-whp

## Overview

Windows Hypervisor Platform backend

## Responsibilities

Provides the Windows Hypervisor Platform integration for the virtualization workspace.

## Architecture

Intended to translate backend-neutral virtualization work into WHP operations.

## Boundaries

WHP is an implementation backend, not a requirement of the guest architecture or shared ABI.

## Status

The crate contains scaffolding only, no WHP integration is implemented.
