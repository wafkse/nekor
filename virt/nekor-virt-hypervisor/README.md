# nekor-virt-hypervisor

## Overview

Shared contracts for native hypervisor backends.

## Responsibilities

Defines the backend marker used by architecture-generic code.

## Architecture

Native KVM and WHP APIs live in their respective crates and implement this contract for supported
architectures.

## Boundaries

Backend resource ownership and execution semantics remain in the native backend crates.

## Status

KVM implements the x86-64 backend marker. WHP has no safe backend resource type yet.
