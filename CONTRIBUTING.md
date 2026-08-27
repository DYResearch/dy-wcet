<!--
SPDX-License-Identifier: Apache-2.0 OR MIT
SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
-->

# Contributing

## The most useful thing you can send

A task set where this crate gives the wrong answer.

Not a bug report about a panic — a set where the number is plausible and wrong.
Those are the failures this exists to prevent, and they are the ones that are
hard to find alone.

If you have one, open an issue with the set and the answer you believe is
correct, derived. If the derivation is right, it becomes a test in
`tests/on_paper.rs` with your reasoning in the comment, and your name on the
commit.

## What a test here looks like

Every expected value in `tests/on_paper.rs` is derived in the comment above it,
iteration by iteration. A test asserting a number without showing where the
number came from is a test that pins the implementation rather than the
analysis — and if the implementation is wrong, such a test protects the bug.

```rust
/// ```text
/// R = 2   ⌈2/4⌉·1 + ⌈2/6⌉·2 = 1 + 2  →  5
/// R = 5   ⌈5/4⌉·1 + ⌈5/6⌉·2 = 2 + 2  →  6
/// R = 6   ⌈6/4⌉·1 + ⌈6/6⌉·2 = 2 + 2  →  6   ← fixed point
/// ```
#[test]
fn three_tasks_settling_at_six() { … }
```

## Before opening a pull request

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --target thumbv7em-none-eabihf
./audit.sh
```

The fourth matters because the crate claims `no_std`, and a claim nobody builds
against a bare-metal target is a claim nobody checked.

The fifth is newer. `audit.sh` checks every number this repository states in
prose against the source that is supposed to back it — test counts, the task
limit, the dependency count, the version in three files, the anchors, the
licence text in the published archive. It was added in 0.1.2 and it found four
things wrong on its first run. If you change a number in the README, that
script is what tells you which other file disagrees with you.

## What will not be merged

**Floating point.** Anywhere. The determinism guarantee is the crate's only
distinguishing property and a single `f64` ends it.

**Unwrapped arithmetic.** Every add and multiply on the interference path is
checked. A wrapping sum converts an unschedulable set into a schedulable-looking
one, which is the one direction an error must never go.

**A capped loop returning its last value.** Non-convergence is an answer, and
the answer is `Unschedulable`.

**Dependencies**, unless something genuinely cannot be done without them.

## Licence

Contributions are Apache-2.0 OR MIT, matching the crate. By opening a pull
request you agree to that.

---

<sub>© 2026 Denis Yermakou — DY Research · connect@axonos.org</sub>

