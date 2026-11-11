# nekor-virt-address

## Overview

Guest addresses

## Responsibilities

Defines semantic guest address types shared by virtualization backends.

## Architecture

`Gpa` preserves the guest physical address domain without applying a machine's configured width.

## Boundaries

Machine-specific address checks and guest memory mappings belong to their owning components.

## Status

Provides the shared `Gpa` type.
