# Changelog

## [0.1.0] — 2026-08-17

First release.

Response-time analysis for a fixed-priority task set, in integer arithmetic
throughout. Twelve tests, including the three failure modes the crate exists
for: a set above full utilisation returning `Unschedulable` rather than a large
number, an overflow reported rather than wrapped, and the same set giving
identical bits across a thousand runs.

The example in the README returns 300 where the obvious guess is 400. That case
is in the tests, because the window that counts is the response time and not the
period, and an implementation using the period passes a naive test and fails a
real system.

---

<sub>SPDX-License-Identifier: Apache-2.0 OR MIT · Copyright (c) 2026 Denis
Yermakou <connect@axonos.org> — DY Research</sub>

