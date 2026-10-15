# nekor-virt-commit

## Overview

Architectural commit

## Responsibilities

Defines publication of prepared architectural state.

## Architecture

Intended to own the commit boundary after provisional effects have been prepared.

## Boundaries

It does not own effect preparation, device state, or host backend policy.

## Status

The crate contains scaffolding only, commit contracts remain planned.
