# nekor-virt-kvm

## Overview

Linux KVM backend

## Responsibilities

Provides the Linux KVM integration for the virtualization workspace.

## Architecture

Intended to translate backend-neutral virtualization work into Linux KVM operations.

## Boundaries

KVM is an implementation backend, not a requirement of the guest architecture or shared ABI.

## Status

The crate contains scaffolding only, no KVM integration is implemented.
