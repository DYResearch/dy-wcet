# Embassy #6528 — RP2350 `embassy-time` Lost-Alarm Timing Analysis

**Target:** RP2350 / `embassy-rp` / `embassy-time`
**Class:** Timing / real-time execution / hardware alarm race
**Analysis:** DYResearch — `dy-wcet`
**Confidence:** High for the observed failure chain · Medium for the exact hardware race mechanism

> This is a source- and reasoning-level analysis. The hardware register state at the instant of failure was **not** captured in the original report, so the specific arming race is presented as a **hypothesis**, not a proven root cause. What is strongly supported by the report, and what is not, is stated explicitly in §"What is proven vs. what is not".

---

## 1. Executive finding

Embassy issue #6528 reports an intermittent RP2350 failure in which **all `embassy-time` timers stop being serviced while the underlying hardware timer counter keeps advancing normally**.

Reported system characteristics:

- RP2350A at 150 MHz, both cores active
- Core 0 uses `embassy-time`; core 1 runs an independent 8 kHz loop
- `TIMER0` continues advancing throughout
- All timer-driven tasks on core 0 can stop for ~219 seconds
- TCP timeouts stop firing (they depend on `embassy-time`)
- Unrelated network activity can suddenly recover the entire timer queue
- The failure appears after the timer queue becomes relatively quiet

The software state transition described is internally consistent with a **self-sustaining lost-alarm condition**.

---

## 2. Relevant execution path

```
schedule_wake()
    │
    ▼
queue.schedule_wake()
    │  queue changed?
    ▼
next_expiration(now)
    │
    ▼
set_alarm(timestamp)
    ├─▶ TIMER.alarm(0).write_value(timestamp as u32)   // low 32 bits only
    ├─▶ now = self.now()
    └─▶ timestamp <= now ?
            ├─ yes ─▶ disarm / retry
            └─ no  ─▶ return "armed"
```

The driver writes only the **low 32 bits** of the timestamp to the hardware comparator, then performs a software timestamp check. The driver itself notes that the high bits of the timestamp are not checked in hardware.

Consequently, a low-word hardware alarm can fire early after a `2^32 µs` wrap:

$$2^{32}\ \mu s = 4{,}294{,}967{,}296\ \mu s \approx 4294.97\ s \approx 71.58\ \text{min}$$

The IRQ path validates the full 64-bit timestamp before releasing the queue.

---

## 3. Timing model

Let:

- `T` = intended alarm timestamp
- `W` = instant the comparator value is written
- `P` = instant the counter passes the programmed low word
- `N` = instant `self.now()` samples the counter

Intended invariant:

$$T > \text{now} \;\Rightarrow\; \text{alarm is armed} \quad\text{and eventually}\quad \text{counter} \to T \Rightarrow \text{IRQ} \to \texttt{check\_alarm()}$$

The proposed race window is:

$$W < P < N$$

while the software observation of `now()` is close enough to the previous value that `T > now_observed` **even though physically** `counter_physical > T`.

If that state is reachable, the software returns "alarm armed" while the hardware has already missed the equality event. That is the fundamental hazard.

---

## 4. Why equality comparison matters

The RP2350 alarm is **not** a `counter >= target` deadline detector; it matches the programmed low-word comparator on **equality**.

```
counter:  ... 1000 1001 1010 1011 1100 ...
                        ▲
                     target   ← if armed *after* this point is passed,
                                 no match occurs until the 32-bit wrap
```

So `T_low < C_low` does **not** imply an immediate interrupt. The next matching low-word value may occur ~`2^32 µs` (≈ 71.58 min) later for a 1 MHz microsecond timer.

This matters for a real-time scheduler because the software's logical deadline can be only microseconds in the past while the hardware recovery horizon is tens of minutes.

---

## 5. Self-sustaining queue failure

The critical part of #6528 is the interaction between the missed alarm and the integrated timer queue:

```
queue has pending task(s)
        │
        ▼
next_expiration() selects earliest deadline
        │
        ▼
set_alarm()
        │
        ▼
hardware alarm produces no IRQ ──▶ no task wakeup
        │                              │
        │                              ▼
        │                     no task re-schedules its timer
        │                              │
        │                              ▼
        └──────────────────── no new set_alarm()  ──▶  system stays dead
```

Let `Q(t)` = number of queued timer entries and `A(t)` = "hardware alarm armed and able to produce the required IRQ". Normal operation needs:

$$Q(t) > 0 \;\land\; A(t) = 1 \;\Rightarrow\; \exists\, \text{IRQ}$$

The failure state is:

$$Q(t) > 0 \;\land\; A(t) = 0$$

with no internal mechanism to transition `A(t): 0 \to 1`. The state is therefore **absorbing** until an external event triggers a new scheduling operation.

---

## 6. Why unrelated network traffic recovers the system

The reported system uses a W5500 Ethernet controller. An external network event makes a task runnable, which mutates the queue and produces a fresh `schedule_wake()`:

```
external network event
   → task becomes runnable
   → schedule_wake()
   → queue changes
   → next_expiration()
   → set_alarm()          // hardware alarm re-armed
   → overdue timers become visible
   → many tasks wake at once
```

This matches the reported "everything resumes at once" behavior, and is strong evidence for a **scheduler recovery dependency on an unrelated external event**. It does not, by itself, prove the exact hardware arming race.

---

## 7. Quantitative evidence (independent counter)

Core 1 runs a producer at `f = 8000 Hz`, i.e. a period of `125 µs`. During the freeze, the SPSC ring overflow counter increased by ≈ `1,751,967` samples. The implied freeze interval:

$$\Delta t = \frac{1{,}751{,}967}{8000} \approx 219.0\ s \approx 3.65\ \text{min}$$

This is important: it shows the **whole MCU did not stop**. Core 1's timing source kept running while core 0's `embassy-time`-driven execution was frozen.

### Why high-frequency activity masks the failure

Before the 8 kHz loop was moved off `embassy-time`, core 1 generated ≈ 8000 scheduling operations/second, i.e. a re-arm roughly every `125 µs`. A transient lost alarm was therefore masked by the next scheduling operation within ~one 125 µs period. This yields a counterintuitive property:

$$\text{higher scheduling activity} \to \text{shorter observed failure} \qquad \text{quiet queue} \to \text{persistent failure}$$

Exactly the kind of failure that average-latency testing misses.

---

## 8. Real-time impact

For a real-time task *i* with release `r_i`, deadline `d_i`, cost `C_i`, response time `R_i`, the requirement is `R_i ≤ d_i − r_i`. Under normal operation:

$$R_i = L_\text{timer} + L_\text{IRQ} + L_\text{scheduler} + C_i$$

The failure replaces a bounded `L_timer ≤ B` with `L_timer = ∞` until an external recovery event, so `R_i → ∞` for every affected timer-dependent task. This is **not** jitter degradation — it is a loss of the bounded worst-case response-time guarantee.

---

## 9. Proposed invariant and fix

The current logic infers validity from `T > now()`, which is weaker than needed. A stronger, hardware-state-based check verifies the alarm's `ARMED` state after programming:

```rust
write_alarm(timestamp);
let now = self.now();

if timestamp <= now {
    if alarm_is_still_armed() {
        // target already passed but alarm never fired: disarm and retry
        disarm();
        return false;
    }
    // Alarm fired during the race window — let the IRQ path complete recovery.
    return false;
}
// timestamp in the future: armed and valid
```

Target invariant:

$$\texttt{set\_alarm}(T) = \text{true} \;\Rightarrow\; \text{hardware alarm state is known to be valid}$$

---

## 10. Independent corroboration

The Raspberry Pi pico-sdk alarm handler uses a materially different architecture: it clears the IRQ, evaluates the earliest deadline against the current timer, processes expired alarms, inserts new ones, programs the next timeout, and **re-checks the deadline before leaving the handler**. The SDK also treats the alarm-configuration interval as a special timing case with explicit semantics for targets that pass during setup.

There is also a separate RP2350 report against pico-sdk 2.3.0 describing a state where the timer advances, the alarm timestamp is already in the past, `ARMED == 0`, and no interrupt is pending, with execution blocked. That report is **distinct** from Embassy #6528 and does not prove a shared root cause — but it establishes that RP2350 alarm-state failures are a real class requiring explicit hardware-state analysis.

---

## 11. What is proven vs. what is not

**Directly supported by the report**

- Timer-driven tasks can stop waking
- The underlying timer counter keeps advancing
- Core 1 stays operational
- The failure can persist for minutes
- Network activity can restore timer-driven execution
- The queue can stay logically populated while no timer wakeup occurs
- The failure is much easier to expose once periodic timer activity is removed

**Plausible but not proven**

- The lost event occurs specifically between the alarm write and the `now()` validation
- The comparator passes the target within that exact window
- The `ARMED` transition is the precise causal distinction
- The proposed modification fully eliminates the failure

The hardware registers were not captured at the instant of failure, so the exact race remains a hypothesis.

---

## 12. Recommended validation experiment

Instrument, at microsecond resolution, the tuple:

$$(t,\ \text{counter},\ \text{alarm},\ \text{ARMED},\ \text{INTR},\ \text{deadline})$$

across `schedule_wake()`, `set_alarm()`, and `check_alarm()`. The goal is to establish whether the system can reach:

$$\text{deadline} < \text{counter} \;\land\; \text{ARMED} = 1 \;\land\; \text{INTR} = 0$$

with no subsequent timer IRQ. That observation converts the current hypothesis into direct hardware evidence.

---

## 13. Engineering conclusion

Embassy #6528 is a strong example of a failure where application logic stays apparently correct, the scheduler queue stays populated, and the hardware timer keeps running — yet the timer **event** is lost, no internal scheduling event exists to recover, and an unrelated external event restores progress.

$$\boxed{\text{Timer correctness} \neq \text{software timestamp correctness}}$$

A WCET or response-time analysis that begins only at task execution is incomplete: the timer-driver boundary is part of the real-time critical path and must be analyzed end to end.

---

**Need this class of analysis on your own system?** Fixed-scope audit → [`AUDIT.md`](../AUDIT.md) · connect@axonos.org

*Analysis by DYResearch (`dy-wcet`). Source references: Embassy issue #6528 and the current `embassy-rp` time driver; Raspberry Pi pico-sdk alarm implementation and API docs; an independent RP2350 pico-sdk alarm-failure report.*
