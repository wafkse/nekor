# nekor-virt-abi

## Overview

Guest/host ABI

## Responsibilities

Defines host-independent protocol representations shared by guest and host virtualization code.

## Architecture

Owns wire-level protocol definitions and their representation, independently of KVM and WHP.

## Boundaries

Transport, backend-specific dispatch, and machine composition belong elsewhere.

## Status

The crate contains scaffolding only, ABI definitions remain planned.
