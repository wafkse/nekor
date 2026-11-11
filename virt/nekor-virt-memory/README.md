# nekor-virt-memory

## Overview

Guest memory

## Responsibilities

Defines guest memory extents measured in bytes and shared by virtualization backends.

## Architecture

Guest regions combine a `Gpa` with a nonzero size and validate that the final byte is representable.

## Boundaries

Host allocation, page alignment, and hypervisor mappings remain backend responsibilities.

## Status

Provides shared guest memory size and region types.
