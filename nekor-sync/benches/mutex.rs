//! Mutex cost and contended throughput.

use core::hint::black_box;
use std::{
    sync::{Barrier, mpsc},
    thread,
    time::Instant,
};

use criterion::{Criterion, criterion_group, criterion_main};
use nekor_sync::mutex::Mutex;

mod support;

/// Benchmark uncontended [`Mutex`] acquisition and release.
fn uncontended(c: &mut Criterion) {
    let unit = Mutex::new(());

    c.bench_function("mutex/uncontended_lock_unlock", |b| {
        b.iter(|| drop(unit.lock()));
    });

    let counter = Mutex::new(0_usize);

    c.bench_function("mutex/uncontended_lock_increment", |b| {
        b.iter(|| {
            let mut guard = counter.lock();

            *guard = guard.wrapping_add(1);
        });
    });

    black_box(*counter.lock());
}

/// Measure wall-clock time for completed operations across persistent workers.
fn contended_case<T, F>(c: &mut Criterion, name: &str, thread_count: usize, mutex: &Mutex<T>, operation: &F)
where
    T: Send,
    F: Fn(&Mutex<T>) + Sync,
{
    let start = Barrier::new(thread_count + 1);
    let finish = Barrier::new(thread_count + 1);

    thread::scope(|scope| {
        let mut senders = Vec::with_capacity(thread_count);

        for _ in 0..thread_count {
            let (sender, receiver) = mpsc::channel::<u64>();

            let start = &start;
            let finish = &finish;

            scope.spawn(move || {
                while let Ok(iterations) = receiver.recv() {
                    start.wait();

                    for _ in 0..iterations {
                        operation(mutex);
                    }

                    finish.wait();
                }
            });

            senders.push(sender);
        }

        c.bench_function(name, |b| {
            b.iter_custom(|iterations| {
                for (worker, sender) in senders.iter().enumerate() {
                    let worker_iterations = iterations / thread_count as u64
                        + u64::from((worker as u64) < iterations % thread_count as u64);

                    sender.send(worker_iterations).expect("benchmark worker stopped");
                }

                // NOTE: Time the start barrier so workers cannot finish before timing begins.
                let started = Instant::now();

                start.wait();
                finish.wait();

                started.elapsed()
            });
        });

        drop(senders);
    });
}

/// Compare empty sections with shared increments, timed per completed operation.
fn contended(c: &mut Criterion) {
    for configuration in support::configurations() {
        configuration.install();

        for thread_count in [2_usize, 4, 8] {
            let unit = Mutex::new(());
            let unit_name = format!(
                "mutex/contended_unit/{thread_count}_threads/{}/{}",
                configuration.monitor_name, configuration.backoff_name
            );

            contended_case(c, &unit_name, thread_count, &unit, &|mutex: &Mutex<()>| {
                drop(mutex.lock());
            });

            let counter = Mutex::new(0_usize);
            let counter_name = format!(
                "mutex/contended/{thread_count}_threads/{}/{}",
                configuration.monitor_name, configuration.backoff_name
            );

            contended_case(c, &counter_name, thread_count, &counter, &|mutex: &Mutex<usize>| {
                let mut guard = mutex.lock();

                *guard = guard.wrapping_add(1);
            });

            black_box(*counter.lock());
        }
    }
}

criterion_group!(mutex_benches, uncontended, contended);
criterion_main!(mutex_benches);
