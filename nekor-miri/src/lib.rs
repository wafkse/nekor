#![no_std]

//! Checked, Miri-only interpreter facilities for Nekor tests.
//!
//! Public modules are compiled only under `cfg(miri)`. The interpreter externs
//! remain private. A static root records intentional process lifetime storage.
//! It does not establish initialization or synchronization of its contents.

#[cfg(miri)]
extern crate alloc;

#[cfg(miri)]
mod ffi;

#[cfg(miri)]
pub mod borrow;
#[cfg(miri)]
pub mod host;
#[cfg(miri)]
pub mod memory;
#[cfg(miri)]
pub mod output;
#[cfg(miri)]
pub mod provenance;
#[cfg(miri)]
pub mod thread;
#[cfg(miri)]
pub mod trace;

#[cfg(test)]
#[cfg(miri)]
mod tests {
    use alloc::{boxed::Box, vec};
    use core::{
        alloc::Layout,
        mem::{self, MaybeUninit},
        ptr::{self, NonNull},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use crate::{borrow, host, memory, output, provenance, thread, trace};

    #[test]
    fn allocation_ownership_and_box_roots() {
        assert!(memory::Allocation::new(Layout::new::<()>()).is_none());
        let layout = Layout::new::<u64>();
        let allocation = memory::Allocation::new(layout).expect("nonzero allocation");
        assert_eq!(allocation.layout(), layout);
        assert_eq!(allocation.pointer().cast::<u64>().as_ptr().align_offset(8), 0);
        allocation.track();
        drop(allocation);

        let rooted = memory::Allocation::new(layout).expect("nonzero allocation");
        let _rooted: NonNull<u8> = rooted.leak();
        let value = memory::leak(Box::new(42_u64));
        assert_eq!(*value, 42);
        let zero = memory::leak(Box::new(()));
        assert_eq!(*zero, ());
        let slice: Box<[u8]> = vec![1, 2, 3].into_boxed_slice();
        assert_eq!(memory::leak(slice), &[1, 2, 3]);
        let empty: Box<[u8]> = vec![].into_boxed_slice();
        assert!(memory::leak(empty).is_empty());
    }

    #[test]
    fn backtrace_capacities_and_names() {
        let mut short = [];
        let required = match trace::Backtrace::capture(&mut short) {
            Err(error) => error.required,
            Ok(_) => unreachable!(),
        };
        assert!(required > 0);

        let mut frames = [MaybeUninit::uninit(); 128];
        let backtrace = trace::Backtrace::capture(&mut frames).expect("frame capacity");
        assert!(!backtrace.frames().is_empty());
        let resolved = backtrace.frames()[0].resolve();
        assert_eq!(mem::offset_of!(trace::MiriFrame, name_len), 0);
        assert_eq!(mem::offset_of!(trace::MiriFrame, filename_len), mem::size_of::<usize>());
        assert_eq!(mem::offset_of!(trace::MiriFrame, lineno), 2 * mem::size_of::<usize>());
        assert_eq!(
            mem::offset_of!(trace::MiriFrame, colno),
            2 * mem::size_of::<usize>() + 4
        );
        assert_eq!(
            mem::offset_of!(trace::MiriFrame, fn_ptr),
            2 * mem::size_of::<usize>() + 8
        );
        assert_eq!(mem::size_of::<trace::MiriFrame>(), 3 * mem::size_of::<usize>() + 8);
        let mut no_name = [];
        let mut no_filename = [];
        let names_required = match resolved.names(&mut no_name, &mut no_filename) {
            Err(error) => error,
            Ok(_) => unreachable!(),
        };
        assert_eq!(names_required.name, resolved.frame().name_len);
        assert_eq!(names_required.filename, resolved.frame().filename_len);

        let mut name = vec![0; names_required.name];
        let mut filename = vec![0; names_required.filename];
        let names = resolved.names(&mut name, &mut filename).expect("name capacity");
        assert!(!names.name.is_empty());
        assert!(!names.filename.is_empty());
    }

    #[test]
    fn diagnostics_alignment_and_output() {
        let value = 7_u64;
        let id = borrow::AllocationId::of(&value).expect("nonzero value");
        id.print(borrow::Visibility::All);
        id.print(borrow::Visibility::Named);
        assert!(borrow::name(&value, 0, "value"));
        assert!(borrow::track(&value).is_some());
        assert!(borrow::AllocationId::of(&()).is_none());
        assert!(!borrow::name(&(), 0, "zero"));
        assert!(provenance::Alignment::new(0).is_none());
        assert!(provenance::Alignment::new(3).is_none());
        let alignment = provenance::Alignment::new(8).expect("power of two");
        provenance::promise(ptr::from_ref(&value).cast::<()>(), alignment);
        provenance::collect();
        output::stdout(b"");
        output::stderr(b"");
    }

    static THREAD_VALUE: AtomicUsize = AtomicUsize::new(0);

    extern "Rust" fn write_value(_: *mut ()) {
        THREAD_VALUE.store(1, Ordering::Release);
    }

    #[test]
    fn thread_joins() {
        THREAD_VALUE.store(0, Ordering::Relaxed);
        // SAFETY: The callback ignores its data and accesses only an atomic static.
        let handle = unsafe { thread::Thread::spawn(write_value, ptr::null_mut()) };
        thread::spin();
        assert!(handle.join());
        assert_eq!(THREAD_VALUE.load(Ordering::Acquire), 1);
    }

    #[test]
    #[ignore = "run separately with MIRIFLAGS=-Zmiri-disable-isolation"]
    fn host_path_capacity() {
        let input = c"/tmp/nekor";
        let mut short = [];
        let required = host::path(input, &mut short).expect_err("short buffer");
        assert!(required.bytes > 0);
        let mut output = vec![0; required.bytes];
        let converted = host::path(input, &mut output).expect("exact capacity");
        assert_eq!(converted.to_bytes(), input.to_bytes());
    }
}
