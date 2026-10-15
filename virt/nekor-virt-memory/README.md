# nekor-virt-memory

## Overview

Guest memory

## Responsibilities

Defines guest addressing, memory ownership, and mapping concepts.

## Architecture

Intended to represent guest memory responsibilities independently from vCPU execution and host backends.

## Boundaries

It does not implement a host memory manager or a concrete hypervisor mapping operation.

## Status

The crate contains scaffolding only, memory contracts remain planned.
