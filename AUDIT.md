<div align="center">

# DY Research — Engagements

### Independent technical intelligence. A written verdict on what is real.

[dyresearch.github.io](https://dyresearch.github.io) · Denis Yermakou, principal · [connect@axonos.org](mailto:connect@axonos.org)

</div>

---

A pitch deck describes a technology. The code decides what it is. Somewhere
between the two sit the claims nobody has checked: the latency that was
measured once and quoted as a bound, the architecture that exists in a diagram
and not in the repository, the verification that turns out to cover one case.

An engagement traces those claims to their evidence — the source, the tests,
the continuous integration, the proofs — and ends in a written verdict that
says what holds, what does not, and where the evidence stops.

Every engagement is carried out by the principal, start to finish. There are no
juniors and no handoffs, and the person who writes the verdict is the person who
read the code.

---

## Snapshot — $5,000

**What does this technology actually do, and what does its evidence support?**

For investors screening a deal and founders preparing for one. Five business
days from confirmed scope.

**Included**

- An architecture map of the system as built, not as described
- A claims-to-evidence matrix: every public technical claim traced to code,
  tests or CI, or marked as unverified
- Engineering health: tests, CI, dependencies, release discipline
- The risks that matter most, ranked, with the evidence for each
- Questions to put to the founders

**You receive** a one-page executive summary, the written report and the
evidence matrix.

---

## Focused Audit — $12,000

**Does this one critical property actually hold?**

For CTOs with a system that has to be right, and investors with one specific
technical doubt. Two to three weeks from confirmed scope.

One property, investigated to the bottom: worst-case timing and response time,
determinism, concurrency and races, the claims made for formal verification, or
the security of a single component.

**Included** — everything in Snapshot, and:

- The execution model reconstructed from source
- Worst-case response times computed with
  [`dy-wcet`](https://github.com/DYResearch/dy-wcet), every iteration shown,
  where timing is the question
- The failure chain traced end to end, with evidence kept apart from hypothesis
- Proposed fixes, each naming what would confirm it and what would rule it out
- A stated confidence boundary: what the evidence proves, and where it stops

**You receive** the audit report, the analysis artefacts needed to reproduce
it, and one written round of questions after delivery.

[**Embassy #6528 — an RP2350 timer that stopped for minutes**](case-studies/embassy-6528.md)
is the standard of delivery for this engagement. It is a complete analysis,
published in full, and not an excerpt of one.

---

## Due Diligence — $25,000

**Is the technology what the company says it is, and what could break the
investment?**

For investors before a term sheet, acquirers and boards. Three to four weeks
from confirmed scope.

**Included** — everything in Focused Audit, applied across the whole system:

- Architecture, code quality and technical debt
- Security posture and the software supply chain
- Testing and verification maturity
- Real-time and determinism claims, wherever the product makes them
- Scalability limits and single points of failure
- An open-source licence inventory — a technical inventory, not a legal opinion
- Every technical claim in the deck, traced to its evidence
- A risk register, each risk rated for severity and likelihood

**You receive** a two-page memo written for an investment committee, the full
report, the risk register, the evidence appendix and one written round of
questions. A mutual NDA is standard.

---

## Where the money goes

Revenue from every engagement funds [AxonOS](https://axonos.org) — an open-source,
deterministic systems layer for neurotechnology — and its path to independent
foundation governance. Your engagement pays for two things:

- **Independent technical intelligence** — the investigation, the evidence
  review and the written verdict you receive.
- **Open infrastructure** — continued development of the AxonOS infrastructure
  layer and its open-source components: the real-time kernel, the signal
  pipeline, and consent enforced below the application layer.

Funding AxonOS never shapes a verdict. If a company under review competes with
AxonOS or builds on it, you are told at scoping, before you commit to anything.

---

## Terms

- **Fixed price, confirmed in writing before any work begins.** Nothing is
  invoiced until the scope is agreed.
- **Invoiced in USD or EUR.** Half on starting, half on delivery.
- **Conflicts of interest are disclosed at scoping**, before either side has
  committed to anything.
- **Expedited timelines on request**, quoted separately.
- **Systems larger than a single product** are scoped and priced individually.

If the question cannot be answered usefully, you hear that before you pay. If
the problem turns out larger than it looked, you hear that before the work
starts, with the rest quoted separately. The scope does not quietly grow, and
neither does the invoice.

---

## What an engagement is not

- **Not investment advice.** The findings are technical. The decision stays yours.
- **Not a certification.** No standard's qualification is issued or implied.
- **Not a warranty.** What you receive is evidence and reasoning, set out so that
  it can be checked.

---

## Requesting an engagement

Send the question in writing: what should be investigated, what you need to
know, and when. No call is required. Scope and price come back in writing.

- **Email** — [connect@axonos.org](mailto:connect@axonos.org)
- **Web** — [dyresearch.github.io](https://dyresearch.github.io)
- **LinkedIn** — [linkedin.com/in/axonos](https://www.linkedin.com/in/axonos)

The work behind the practice is public and can be checked before anything is
bought: [the Radar](https://axonos-bci.github.io/axonos-community-radar/), a
living map of open neurotech; [`dy-wcet`](https://github.com/DYResearch/dy-wcet),
timing analysis that refuses rather than rounds; and
[AxonOS](https://axonos.org), a hard real-time operating system for
brain–computer interfaces.

---
