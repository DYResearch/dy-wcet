// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Properties that must hold for every task set, checked over generated ones.
//!
//! No property-testing crate. A generator is thirty lines of linear
//! congruence, and a dependency tree pulled in to produce pseudo-random `u64`
//! would cost this crate the one thing it advertises. The seed is fixed, so a
//! failure here reproduces on any machine from the line number alone.

use dy_wcet::{Response, Task, TaskSet, Unbounded, MAX_TASKS};

/// Numerical Recipes' LCG. Deterministic, and the constants are published.
struct Rng(u64);

impl Rng {
    fn bits(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }
    fn between(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.bits() % (hi - lo + 1)
    }
}

/// A set that is plausible rather than adversarial: periods and execution
/// times in ranges an embedded system actually uses.
fn generate(rng: &mut Rng, n: usize) -> TaskSet {
    let mut s = TaskSet::new();
    for _ in 0..n {
        let period = rng.between(100, 100_000);
        let wcet = rng.between(1, period.max(2) / 2);
        let jitter = if rng.bits() % 4 == 0 {
            rng.between(0, period / 10)
        } else {
            0
        };
        let blocking = if rng.bits() % 4 == 0 {
            rng.between(0, wcet)
        } else {
            0
        };
        let deadline = rng.between(wcet, period.saturating_mul(2));
        let _ = s.push(
            Task::new(wcet, period)
                .deadline(deadline)
                .jitter(jitter)
                .blocking(blocking),
        );
    }
    s
}

const TRIALS: usize = 4_000;

#[test]
fn a_response_is_never_shorter_than_the_work_it_contains() {
    let mut rng = Rng(0x5EED_0001);
    for _ in 0..TRIALS {
        let s = generate(&mut rng, 6);
        for i in 0..s.len() {
            let task = s.get(i).unwrap();
            if let Some(r) = s.response_of(i).response_time() {
                assert!(
                    r >= task.wcet_us + task.blocking_us + task.jitter_us,
                    "response {r} is below C+B+J for {task:?}"
                );
            }
        }
    }
}

#[test]
fn dropping_a_task_one_priority_level_never_helps_it() {
    // Move the lowest task up one place. Its response time cannot get worse,
    // because the set of things that can preempt it only shrinks.
    let mut rng = Rng(0x5EED_0002);
    let mut compared = 0usize;
    for _ in 0..TRIALS {
        let s = generate(&mut rng, 5);
        if s.len() < 3 {
            continue;
        }
        let last = s.len() - 1;
        let mut lifted = TaskSet::new();
        for (i, t) in s.iter().enumerate() {
            if i == last - 1 {
                let _ = lifted.push(*s.get(last).unwrap());
            }
            if i != last {
                let _ = lifted.push(*t);
            }
        }
        if let (Some(low), Some(high)) = (
            s.response_of(last).response_time(),
            lifted.response_of(last - 1).response_time(),
        ) {
            assert!(
                high <= low,
                "a task got slower by being given higher priority: {low} -> {high}"
            );
            compared += 1;
        }
    }
    assert!(compared > 0, "nothing comparable was generated");
}

#[test]
fn adding_jitter_never_shortens_a_response() {
    let mut rng = Rng(0x5EED_0003);
    for _ in 0..TRIALS {
        let base = generate(&mut rng, 4);
        if base.len() < 2 {
            continue;
        }
        let mut widened = TaskSet::new();
        for t in base.iter() {
            let _ = widened.push(
                Task::new(t.wcet_us, t.period_us)
                    .deadline(u64::MAX)
                    .blocking(t.blocking_us)
                    .jitter(t.jitter_us + 1),
            );
        }
        let mut plain = TaskSet::new();
        for t in base.iter() {
            let _ = plain.push(
                Task::new(t.wcet_us, t.period_us)
                    .deadline(u64::MAX)
                    .blocking(t.blocking_us)
                    .jitter(t.jitter_us),
            );
        }
        let last = plain.len() - 1;
        if let (Some(a), Some(b)) = (
            plain.response_of(last).response_time(),
            widened.response_of(last).response_time(),
        ) {
            assert!(b >= a, "extra jitter shortened a response: {a} -> {b}");
        }
    }
}

#[test]
fn the_same_set_gives_the_same_bits_across_repetition() {
    let mut rng = Rng(0x5EED_0004);
    for _ in 0..TRIALS {
        let s = generate(&mut rng, 8);
        for i in 0..s.len() {
            let first = s.response_of(i);
            for _ in 0..8 {
                assert_eq!(s.response_of(i), first, "same input, different answer");
            }
        }
    }
}

#[test]
fn a_bounded_answer_always_meets_its_deadline_and_an_unbounded_one_never_does() {
    let mut rng = Rng(0x5EED_0005);
    for _ in 0..TRIALS {
        let s = generate(&mut rng, 7);
        for i in 0..s.len() {
            let task = s.get(i).unwrap();
            match s.response_of(i) {
                Response::Bounded(r) => {
                    assert!(r <= task.deadline_us);
                    assert!(s.response_of(i).meets(task.deadline_us));
                }
                Response::Unbounded(Unbounded::ExceedsDeadline(r)) => {
                    assert!(
                        r > task.deadline_us,
                        "ExceedsDeadline({r}) is not past {}",
                        task.deadline_us
                    );
                    assert!(!s.response_of(i).meets(task.deadline_us));
                }
                Response::Unbounded(_) => {
                    assert!(!s.response_of(i).meets(u64::MAX));
                }
            }
        }
    }
}

#[test]
fn audsley_is_optimal_and_not_merely_lucky() {
    // If Audsley returns an ordering, that ordering must actually work.
    let mut rng = Rng(0x5EED_0006);
    let mut found = 0usize;
    for _ in 0..TRIALS {
        let s = generate(&mut rng, 4);
        if let Some(order) = s.optimal_priority_order() {
            let mut arranged = TaskSet::new();
            for &idx in order.iter().take(s.len()) {
                let _ = arranged.push(*s.get(idx).unwrap());
            }
            assert!(
                arranged.is_schedulable(),
                "Audsley proposed an ordering that misses"
            );
            found += 1;
        }
    }
    assert!(
        found > 0,
        "the generator produced nothing Audsley could order"
    );
}

#[test]
fn sensitivity_is_the_last_value_that_fits_and_not_one_more() {
    let mut rng = Rng(0x5EED_0007);
    let mut checked = 0usize;
    for _ in 0..1_000 {
        let s = generate(&mut rng, 3);
        if !s.is_schedulable() || s.is_empty() {
            continue;
        }
        for i in 0..s.len() {
            let Some(extra) = s.max_wcet_increase(i) else {
                continue;
            };
            let task = *s.get(i).unwrap();
            let build = |c: u64| {
                let mut t = TaskSet::new();
                for (k, orig) in s.iter().enumerate() {
                    let mut o = *orig;
                    if k == i {
                        o.wcet_us = c;
                    }
                    let _ = t.push(o);
                }
                t
            };
            assert!(
                build(task.wcet_us + extra).is_schedulable(),
                "the reported increase does not fit"
            );
            if task.wcet_us + extra < task.deadline_us {
                assert!(
                    !build(task.wcet_us + extra + 1).is_schedulable(),
                    "one more would also have fitted, so the answer was not maximal"
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 0, "no schedulable set was generated to probe");
}

#[test]
fn the_set_limit_holds_under_generation() {
    let mut rng = Rng(0x5EED_0008);
    for _ in 0..200 {
        let s = generate(&mut rng, MAX_TASKS + 5);
        assert!(s.len() <= MAX_TASKS);
    }
}
