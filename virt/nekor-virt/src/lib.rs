#![no_std]
#![doc = include_str!("../README.md")]

extern crate alloc;

pub extern crate nekor_virt_abi as abi;
pub extern crate nekor_virt_access as access;
pub extern crate nekor_virt_address as address;
pub extern crate nekor_virt_arch as arch;
pub extern crate nekor_virt_hypercall as hypercall;
pub extern crate nekor_virt_ioport as ioport;
pub extern crate nekor_virt_memory as memory;
pub extern crate nekor_virt_mmio as mmio;
pub extern crate nekor_virt_model as model;
pub extern crate nekor_virt_provide as provide;

pub mod hypervisor;
