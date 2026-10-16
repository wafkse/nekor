# nekor-virt-peripheral

This crate models guest peripheral operations and their architectural consequences. Its scope includes processor-integrated facilities, such as local APIC interfaces, as well as external devices defined by the guest platform.

The execution backend supplies an architectural input to [`Peripheral::action`](crate::peripheral::Peripheral::action), which delegates to the implementation selected by [`Map`](crate::mapped::Map). Each [`Mapping`](crate::mapped::Mapping) associates a peripheral, backend, and architecture with a state representation and operation delegate, allowing one peripheral interface to serve multiple backends. The caller supplies the state instance and controls its publication.

Successful and unsuccessful operations can both produce guest-visible effects. For example, an invalid register access may require an exception, so the caller must stage either outcome through [`Effect::stage`](crate::effect::Effect::stage). The transaction's [`Stage`](crate::effect::Stage) implementation records those consequences in provisional state and reports preparation failures separately from guest architectural errors.

[`Batch`](crate::effect::transact::Batch) begins a transaction, and [`Prepare`](crate::effect::transact::Prepare) resolves the fallible requirements for publication before producing a [`Commit`](crate::effect::transact::Commit)-capable value. A staging failure must block preparation until explicit recovery restores consistency. Because either the transaction or its prepared value can be abandoned, implementations must preserve authoritative state until commit. Publication visibility and the participating state are defined by the concrete transaction implementation.

Guest architecture identity comes from `nekor-virt-model`, while `nekor-virt-hypervisor` supplies backend identity. This crate provides the peripheral and transaction contracts and a runnable mapping example. Concrete device mappings and transaction engines remain implementation work.
