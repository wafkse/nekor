# nekor-virt-doorbell

## Overview

Guest/host doorbell

## Responsibilities

Defines guest-facing doorbell protocol concepts for host notification.

## Architecture

Intended to represent a shared guest/host signaling contract without binding it to a hypervisor framework.

## Boundaries

It does not implement a transport, host dispatcher, or hypercall semantics.

## Status

The crate contains scaffolding only, doorbell definitions remain planned.
