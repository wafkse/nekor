# eustace-mmio

## Overview

MMIO access

## Responsibilities

Defines guest memory-mapped access descriptions shared by virtualization backends.

## Architecture

Access records preserve guest physical addresses, direction, and transfer width.

## Boundaries

Device models and backend exit transport remain separate responsibilities.

## Status

Provides shared x86 MMIO access types.
