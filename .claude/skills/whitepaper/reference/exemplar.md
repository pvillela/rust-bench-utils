# whitepaper — annotated exemplar

A short, deliberately **neutral-domain** model of the target style. The content (a made-up
comparison of two cache designs) is not the point — the **shape and the standards** are. Margin
notes in `> ← …` blockquotes call out which rule each part satisfies; they are commentary, not
part of a real report.

The real thing would be longer and its evidence real. This is trimmed to fit on one screen while
still exercising every element of the spec: framing intro, setup, theory-then-observed evidence
with a table, numbered bold-lead findings, a labeled conjecture, recommendations, and a decisive
bottom line.

---

# Assessment: Write-Through vs. Write-Back Caching for the Session Store

This report compares two caching strategies for the session store — **write-through** (every
write updates cache and backing store synchronously) and **write-back** (writes hit the cache and
flush to the store asynchronously) — on **tail write latency** and **data-loss exposure**. It is
a follow-up to `Assessment_Session_Store_Baseline.md` (report #1), which established the
uncached baseline; all latencies here are relative to that baseline's p99 of **12.0 ms**.

> ← **Framing intro**: states the question, defines the two terms **at first use and in bold**,
> names the two axes, situates against a prior report, and fixes the notation (relative to a
> stated baseline). Jargon is kept to terms a general-STEM reader knows.

## 1. Setup

**Workload.** 50k session writes replayed from production capture, 5% of them read-modify-write
hot keys. **Environment.** Single node, store on local NVMe, cache in RAM. **Metric.**
per-write latency, reported as p50 / p99 / p99.9 over 5 runs; **data-loss window** = worst-case
unflushed writes at a crash, in operations. Harness: `bench_cache_strategies.rs`; raw output:
`results/cache_strategy.csv`.

> ← **Setup** establishes everything the evidence depends on, and **names the script and the
> raw-output file** so every number below is traceable.

## 2. What theory predicts

Write-back should win on latency by exactly the store's write cost, since it removes the
synchronous store write from the critical path: we expect its p99 to approach the cache's own
write latency (~0.2 ms) while write-through stays bounded below by the store write (~2 ms).
Write-back should lose on data-loss exposure in proportion to its flush interval: a flush every
`f` ms exposes up to `f × throughput` unflushed writes.

> ← **Theory first, prediction quantified.** This gives the observed numbers something to be
> checked against — and sets up the consistency check.

## 3. Results

All latencies in ms, relative baseline p99 = 12.0 ms; best per column in **bold**.

| Strategy | p50 | p99 | p99.9 | data-loss window (ops) |
|---|---|---|---|---|
| write-through | 2.1 | **3.4** | 6.8 | **0** |
| write-back (f=10 ms) | **0.2** | 0.9 | 2.1 | ~1,900 |
| write-back (f=1 ms) | **0.2** | 1.1 | 3.9 | ~190 |

Consistency checks passed: write-through p99 (3.4 ms) sits just above the ~2 ms store-write
floor as predicted; write-back p50 matches the ~0.2 ms cache-write cost; the data-loss window
scales ~10× with a 10× longer flush interval (~190 → ~1,900), as the `f × throughput` model
predicts.

> ← **Results in a table**; **observed reconciled against §2's predictions** in an explicit
> consistency-checks note. Numbers agree with theory, which is stated, not assumed.

## 4. Assessment

1. **Write-back cuts tail write latency by ~3.7× but never reaches zero data-loss.** Its p99
   (0.9–1.1 ms) beats write-through's 3.4 ms because the synchronous store write is off the
   critical path, exactly as predicted — but any non-zero flush interval leaves a crash-loss
   window, so the two strategies are not on the same safety footing.
2. **Tightening the flush interval trades latency tail for safety, not p50.** Going from f=10 ms
   to f=1 ms shrinks the loss window 10× (~1,900 → ~190 ops) and leaves p50 unchanged (0.2 ms),
   but inflates p99.9 from 2.1 to 3.9 ms as flushes contend with writes. The dial moves the tail
   and the safety window together, not the median.
3. **Write-through's tail is the store's tail.** Its p99.9 of 6.8 ms tracks NVMe write-latency
   spikes; no cache tuning removes it, because every write waits on the store by construction.

> ← **Numbered findings, each led by a bold claim sentence** carrying a **magnitude**, then the
> evidence. Non-hedging: "cuts by ~3.7×", not "seems faster".

*Conjecture (not tested here):* a hybrid that flushes synchronously only for the 5% hot keys
might capture most of write-back's latency win while bounding loss on the keys that matter — but
this was **not measured**, and hot-key flush contention could erase the gain. Open question for a
follow-up.

> ← **Conjecture explicitly labeled** and quarantined from the verified findings above. It is
> never allowed to read as an established result.

## 5. Recommendations

1. **Default to write-back at f=1 ms** for latency-sensitive session traffic where a ~190-op
   crash-loss window is tolerable — it delivers the p50/p99 win at the smallest safety cost
   measured.
2. **Use write-through where any session loss is unacceptable** (e.g. auth tokens); pay the
   ~3.4 ms p99 as the price of a zero loss window.
3. **Prototype the hot-key hybrid before adopting it** — the conjecture in §4 is promising but
   unmeasured; treat it as a spike, not a plan.

## 6. Bottom line

**Ship write-back at a 1 ms flush for the general session store, and pin write-through for
loss-intolerant keys.** The latency win is real and matches theory; the only open decision is
per-key safety tolerance, which the flush interval — not the strategy choice alone — controls.

> ← **Bottom line states the decision** and the one remaining judgment call. It does not
> re-summarize §§1–5.
