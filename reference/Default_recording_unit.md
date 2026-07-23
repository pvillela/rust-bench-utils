# Default recording unit

## Decision

The crate's default recording unit should be `LatencyUnit::sub_sec(11)` — one recording unit
per 10 picoseconds. (The code constant `BenchCfg::DEFAULT_RECORDING_UNIT`, `src/bench_cfg.rs:28`,
currently reads `LatencyUnit::NANO`; changing it to `LatencyUnit::sub_sec(11)` is the follow-up
this rationale supports.)

`LatencyUnit` (`src/latency.rs:203-222`) is a `u8` newtype: `LatencyUnit::sub_sec(n)` represents
`10^-n` seconds, i.e. `10^n` recording units per second. `NANO == sub_sec(9)`, `MICRO ==
sub_sec(6)`, `MILLI == sub_sec(3)`, `SEC == sub_sec(0)`, `PICO == sub_sec(12)`. This document
motivates choosing `n = 11` over the current `n = 9` (`NANO`).

The two knobs that jointly determine a `BenchOut`'s histogram resolution are independent:
**`sigfig`** (`BenchCfg::DEFAULT_SIGFIG = 4`, `src/bench_cfg.rs:35`) fixes the best-case relative
precision the histogram can ever deliver, everywhere in its range. **The recording unit** decides
*how small a latency has to be, in real time, before it is too small to reach that best-case
precision.* This document is about the second knob only; the reasoning behind `sigfig = 4` is
covered in [`Q_n_resolution_and_outlier_analysis.md`](Q_n_resolution_and_outlier_analysis.md).

## Givens

- `BenchOut`'s histogram is an HDR histogram (`hdrhistogram::Histogram<u64>`), built by
  `new_hdrhist` (`src/bench_out.rs:11-17`):

  ```rust
  pub(crate) fn new_hdrhist(hist_high: u64, hist_sigfig: u8) -> Histogram<u64> {
      let mut hist = Histogram::<u64>::new_with_max(hist_high, hist_sigfig)
          .expect("should not happen given histogram construction");
      hist.auto(true);
      hist
  }
  ```

  `new_with_max(high, sigfig)` is `new_with_bounds(1, high, sigfig)` — so the histogram's `low` is
  always `1` and `unit_magnitude` is always `0`, regardless of recording unit. `hist.auto(true)`
  makes the histogram auto-resize if a value exceeds `high`, so `high` is only an *initial* sizing
  hint, not a hard cap.
- The caller, `BenchOut::new` (`src/bench_out.rs:81-84`), sets `high` to 10 seconds expressed in
  the configured recording unit:

  ```rust
  let high_latency = FpSeconds::from_secs(10);
  let hist_high = cfg.recording_unit().value_from_fpsecs(high_latency);
  let hist = new_hdrhist(hist_high, cfg.sigfig());
  ```

  So with `NANO`, the initial `high` is `1e10`; with `sub_sec(10)`, `1e11`; with `sub_sec(11)`,
  `1e12`.
- The default `sigfig` is `4` (`src/bench_cfg.rs:35`), which fixes `sub_bucket_count = 32,768` and
  bucket 0's span as `0..32,767` at unit resolution
  (see [`Q_n_resolution_and_outlier_analysis.md`](Q_n_resolution_and_outlier_analysis.md), "How
  `sigfig` fixes `sub_bucket_count`").

## How resolution actually depends on the recording unit

For a recorded value with sub-bucket index `m` (with `unit_magnitude = 0`, `m` is just the value
itself while it's within bucket 0), relative precision is `1/m`
([`Q_n_resolution_and_outlier_analysis.md`](Q_n_resolution_and_outlier_analysis.md), "What this
means for resolution"). Two regimes follow:

- **Bucket 0's lower half** (`m` from 1 to 16,383): `1/m` is unbounded and *improves* as `m`
  grows — this is the only region where the choice of recording unit matters.
- **The floor** (`m` from 16,384 to 32,767 — bucket 0's upper half, and the equivalently-indexed
  range of every higher bucket `n ≥ 1`, since relative width is scale-invariant): `1/m` is boxed
  into `[1/32768, 1/16384] ≈ 3.1×10⁻⁵ … 6.1×10⁻⁵` at `sigfig = 4`. This floor is set by `sigfig`
  alone; no recording unit can push relative precision any finer than it.

**Consequence:** multiplying the recording unit by 10 multiplies every recorded value's index by
10, which only helps while that index is still below the floor (`m < 16,384`). It moves small
latencies up through bucket 0's lower half toward the floor; it does nothing for latencies already
at or past the floor, and buys nothing beyond it but wasted histogram range (and memory, see
below). So the recording-unit question reduces to: **for the smallest batch latency you actually
care about, is its index comfortably below 16,384 but as close to it as practical?**

## Design criterion and machine-speed assumptions

Two constraints bound how small a *batch* latency can meaningfully get:

1. **Measurement overhead must be negligible relative to what's measured.** On a computer similar
   to mine, overhead is ~10–20 ns; it becomes negligible once latency reaches ~10 μs. On a machine
   ~10× faster, overhead is ~1–2 ns, negligible above ~1 μs. On a machine ~100× faster (a
   generous extrapolation, not separately measured), overhead would be ~0.1–0.2 ns, negligible
   above ~0.1 μs.
2. **Batching pushes recorded latencies up to (at least) that overhead-negligible floor.** When a
   single call's latency is too small for overhead to be negligible, `bench_run`'s `batch`
   parameter groups `n` calls per recorded observation so the *recorded* (batch) latency clears
   the floor. This is also why sub-nanosecond recording units are meaningful at all: a batch
   latency is a sum/mean over many calls, not a single clock read, so it can legitimately carry
   sub-nanosecond-scale information once divided back out.

Combining the two: the smallest latency we need good resolution for is the "batching floor" — the
overhead-negligible latency for the target machine speed. Requiring that floor's index be
comfortably below 16,384 — say, ≥ 10,000, giving relative precision ≤ 10⁻⁴, one order of magnitude
better than the ~10⁻³ that bucket-0's low end alone would give — sets a lower bound on `n`.

## Unit comparison

| Recording unit | Batching floor → index (at `sigfig = 4`) | Relative precision at the floor | Machine regime covered |
|---|---|---|---|
| `LatencyUnit::NANO` (`sub_sec(9)`) | 10 μs → 10,000 | ~10⁻⁴ | reference machine |
| `LatencyUnit::sub_sec(10)` | 1 μs → 10,000 | ~10⁻⁴ | ~10× faster |
| `LatencyUnit::sub_sec(11)` | 0.1 μs → 10,000 | ~10⁻⁴ | ~100× faster |
| `LatencyUnit::PICO` (`sub_sec(12)`) | 10 ns → 10,000 | ~10⁻⁴ | ~1,000× faster (not chosen) |

Each row is self-consistent (10⁻⁴ precision at each regime's own batching floor) but *only* the
chosen unit gives that precision for latencies from *all* covered regimes simultaneously — a
batch latency of 0.1 μs recorded in `NANO` lands at index 100 (relative precision 10⁻²), two
orders of magnitude worse. Choosing `sub_sec(11)` means every batch latency from 0.1 μs up carries
≤10⁻⁴ relative precision — comfortable headroom for machines up to ~100× faster than the
reference — while every latency ≥ 10 μs still sits at the sigfig-4 floor (~3–6×10⁻⁵), exactly as
it would under `NANO`. There is no precision *cost* anywhere in the range for adopting the finer
unit; the only cost is memory.

`sub_sec(12)` (`PICO`) is not chosen: it buys headroom for a 1,000×-faster machine at further
memory cost, well past any machine this crate is likely to run benchmarks on, and 100× headroom
over the reference machine (`sub_sec(11)`) is already generous.

## Memory cost

The histogram's `counts` array is `hdrhistogram`'s only large allocation, and its size is
`(bucket_count + 1) × (sub_bucket_count / 2)` `u64` entries — a function of `sigfig` and `high`
only, **independent of how many observations are recorded**
(see [`Histogram_vs_Vec_sample_storage.md`](Histogram_vs_Vec_sample_storage.md) §3.1, which
reports ~2.75 MB for the current `NANO` configuration). At `sigfig = 4`
(`sub_bucket_count = 32,768`) and the initial 10-second `high` used by `BenchOut::new`:

| Recording unit | `high` (units) | `bucket_count` | `counts` length | Memory |
|---|---|---|---|---|
| `NANO` (`sub_sec(9)`) | 1e10 | 20 | 344,064 | ~2.75 MB |
| `sub_sec(10)` | 1e11 | 23 | 393,216 | ~3.15 MB (+14%) |
| `sub_sec(11)` | 1e12 | 26 | 442,368 | ~3.54 MB (+29%) |

(This is the initial allocation only; `hist.auto(true)` lets it grow further if an observed value
ever exceeds 10 seconds, at any recording unit.) Note there is no separate "values array" —
`hdrhistogram` stores counts only, indexed by bucket/sub-bucket position; a value is recovered
from its index arithmetically, not looked up.

## Conclusion

Adopt `LatencyUnit::sub_sec(11)` as the default recording unit. It keeps relative precision at
≤10⁻⁴ for every batch latency from 0.1 μs upward — comfortable headroom for machines up to ~100×
faster than the reference machine used to estimate measurement overhead — while every latency at
or above the sigfig-4 floor gets the same best-case precision (~3–6×10⁻⁵) it would under `NANO`.
The cost is a ~29% larger initial histogram allocation (~3.54 MB vs. ~2.75 MB), which is
negligible in absolute terms and independent of how many samples are ultimately recorded. The code
constant `BenchCfg::DEFAULT_RECORDING_UNIT` (`src/bench_cfg.rs:28`), currently `LatencyUnit::NANO`,
should be updated to `LatencyUnit::sub_sec(11)` to match.
