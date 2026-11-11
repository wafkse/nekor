# nekor-virt-ioport

## Overview

I/O port access

## Responsibilities

Defines x86 guest port addresses and operation descriptions shared by virtualization backends.

## Architecture

Access records retain direction, element width, port, and positive transfer count.

## Boundaries

Device emulation and backend exit handling remain in their owning components.

## Status

Provides shared port and access types.
