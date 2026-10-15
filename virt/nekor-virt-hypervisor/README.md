# nekor-virt-hypervisor

## Overview

Hypervisor abstraction

## Responsibilities

Provides a home for host virtualization capability abstractions shared by backend integrations.

## Architecture

Intended to keep backend-neutral host concepts distinct from Linux and Windows implementation crates.

## Boundaries

It does not implement guest architecture, machine policy, or a concrete backend.

## Status

The crate contains scaffolding only, abstractions remain planned.
