# Nekor

Nekor is an async-first `no-std`, `no-alloc` unikernel written in Rust.

## Goals

As of writing, Nekor aims to deliver:

- A lock-free, SMP-capable global `O(1)` task scheduler with explicit task priority.
- True allocation-less event-driven concurrency.
- Hassle-free static allocation for all tasks and their required resources (e.g., buffers, handles, etc.).
- Multi-architecture support: `x86-64`, `armv7+`, `aarch64`, `riscv` (`*A` extension is required).
- Out-of-the-box usermode support for simulator usage (Native and paravirtualized).
- Externally-verifiable behavior, including memory IO and other "magical" memory accesses.

Additionally, Nekor plans to:

- Provide exhaustive test fixtures for all major kernel components.
- Run all test cases under Miri.
- Perform model checking with Kani on critical algorithmic code paths.

Test cases and Miri runs are performed in every CI cycle. Due to longer execution times, Kani is run on each git tag (and thus release candidate).

This is the required feature baseline for the first usable release.

# Licensing

© 2025 - Present W. Frakchi

Nekor is dual-licensed under:

- [GNU Affero General Public License v3.0](LICENSE) (AGPL-3.0-only), **or**
- a **commercial license** by arrangement.

You may choose either license according to your needs.

For commercial use, private deployment, certification, or miscellaneous support, please contact: `license.diner324@passinbox.com` (*initial contact only - replies will come from a verified address.*)
