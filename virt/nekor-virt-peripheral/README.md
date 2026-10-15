# nekor-virt-peripheral

## Overview

Virtual peripherals

## Responsibilities

Defines architecture-aware sans-I/O peripheral state transitions.

## Architecture

Intended to own reusable device transition contracts while leaving transport and scheduling to callers.

## Boundaries

It does not own concrete x86 devices, host framework state, or machine composition.

## Status

The crate contains scaffolding only, peripheral contracts remain planned.
