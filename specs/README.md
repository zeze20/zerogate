# ZeroGate Formal Specifications

This directory documents ZeroGate's TLA+ formal specifications and how to run
them. The specification sources live under [`../formal/tla/`](../formal/tla/).

> Honest scope: these specs model **abstract designs** and are **model-checked**
> with TLC. They do not prove the Rust implementation line-by-line, nor the
> correctness of the Linux kernel, the eBPF verifier, the AF_XDP runtime, the
> NIC/DMA hardware, or the compiler. See
> [`../docs/FORMAL_ASSURANCE.md`](../docs/FORMAL_ASSURANCE.md).

## Specifications that exist today

| Spec | Source | Models | Status |
|------|--------|--------|--------|
| FrameOwnership | [`../formal/tla/FrameOwnership.tla`](../formal/tla/FrameOwnership.tla) + [`.cfg`](../formal/tla/FrameOwnership.cfg) | MR10 UMEM frame ownership state machine | Model-checked (TLC PASS, N=3, N=4) |

`FrameOwnership.tla` currently proves **ownership safety properties** (legal
transitions, free-list consistency, no duplicate ownership, Tx-not-free-before-
completion). It does **not** yet model the full queue lifecycle — that is future
`QueueLifecycle.tla` work (see below).

## How to run locally

The runner script is [`../scripts/run_tla.sh`](../scripts/run_tla.sh). It needs:

- A Java 11+ runtime (`java` on `PATH`, or `$JAVA`).
- `tla2tools.jar` — download once from
  <https://github.com/tlaplus/tlaplus/releases> and either place it at the repo
  root (`tla2tools.jar`) or export `TLA2TOOLS_JAR=/path/to/tla2tools.jar`. See
  [`../docs/DEVELOPMENT.md`](../docs/DEVELOPMENT.md) for full local setup.

The script does **not** download anything itself, so it stays deterministic and
offline-friendly. If tooling is missing it exits non-zero with instructions — it
never fakes a pass.

```bash
# quick: N=3 and N=4 (this is what CI runs)
./scripts/run_tla.sh quick

# extended: N=3, N=4, plus a deeper N=5 exploration
./scripts/run_tla.sh extended

# run FrameOwnership with the committed default config (N=3)
./scripts/run_tla.sh frame-ownership

./scripts/run_tla.sh --help
```

## What N means

`N` is the number of UMEM frames in the modeled pool. TLC explores **every**
reachable state of the ownership state machine for that fixed frame count.

| N | Distinct states | Search depth | Notes |
|---|-----------------|--------------|-------|
| 3 | 343 | 19 | Default; sufficient to exercise all transitions and invariants |
| 4 | 2401 | 25 | Quick-mode upper bound; more interleavings |
| 5 | 16807 | — | Extended mode; deeper exploration |

Increasing `N` multiplies the state space (roughly `7^N` before reachability
pruning). Because the invariants are universally quantified over frames and the
transitions are uniform across frames, small `N` already exercises the relevant
ownership interactions; larger `N` increases confidence but not coverage of new
behavior shapes.

### N=8 / symmetry

`N=8` and symmetry-reduced checking are **not implemented yet**. The current
spec has no symmetry set, so an `N=8` run would be expensive without adding much.
The runner intentionally does **not** offer N=8 and prints a note in extended
mode rather than pretending it is supported. Adding a `Permutations`/symmetry set
is tracked as future spec work.

## How CI runs the specs

- **GitLab CI (active):** the `tla_model_check` job in
  [`../.gitlab-ci.yml`](../.gitlab-ci.yml) (stage `formal`) uses the
  `eclipse-temurin:21-jdk` image, downloads `tla2tools.jar`, and runs
  `./scripts/run_tla.sh quick`.
- **GitHub Actions (active):** [`../.github/workflows/tla.yml`](../.github/workflows/tla.yml)
  installs Temurin JDK 21, downloads the pinned `tla2tools.jar`, sets
  `TLA2TOOLS_JAR`, and runs `./scripts/run_tla.sh quick`. It triggers on
  `pull_request` (any base — stacked PRs included), `workflow_dispatch` (with a
  `mode` input for `extended`), and `push` to `main`.

Both jobs run **real** TLC; they do not echo a fake success.

## What a TLC failure means

If TLC finds a state violating an invariant, it prints a **counterexample
trace** (the sequence of actions reaching the bad state) and exits non-zero,
which fails the job. A failure means either:

1. the spec was changed in a way that broke an invariant, or
2. a real ownership-logic design flaw was introduced.

Investigate the trace before changing the invariant. Do not weaken an invariant
to make TLC pass unless the invariant itself is proven wrong.

## Rust ↔ TLA+ mapping (high level)

| Rust | TLA+ |
|------|------|
| `FrameState` enum (7 states) | `FrameStates` set |
| `FramePool.states: Vec<FrameState>` | `state` function `Frames -> FrameStates` |
| `FramePool.free_list: VecDeque<usize>` | `free` set |
| `FrameState::can_transition_to` | `CanTransition(from, to)` |
| `allocate_for_fill` / `mark_kernel_owned` / `mark_rx` / `acquire_user` / `recycle_to_fill` / `submit_tx` / `complete_tx` / `release_completion` | actions `AllocateForFill` / `MarkKernelOwned` / `MarkRx` / `AcquireUser` / `RecycleToFill` / `SubmitTx` / `CompleteTx` / `ReleaseCompletion` |

Full per-method mapping is in
[`../docs/FRAME_OWNERSHIP.md`](../docs/FRAME_OWNERSHIP.md).

## Current verified scope

- Only legal lifecycle transitions are reachable.
- The free set always equals the set of `Free` frames (`FreeListMatchesState`).
- No non-Free frame is ever in the free set.
- A `Tx` frame is never free before passing through `Completion`.
- No duplicate ownership within the model.

## Out of scope (today)

- Queue lifecycle (fill/RX/TX/completion ring interactions).
- Liveness/progress properties (currently safety-only).
- Real kernel, eBPF verifier, AF_XDP runtime, hardware, compiler correctness.
- The Rust implementation verified line-by-line.

## Future: `QueueLifecycle.tla`

A future MR (MR11.1) will add `QueueLifecycle.tla` modeling the queue worker
interacting with fill/RX/TX/completion rings. It is expected to **compose with
or refine** `FrameOwnership.tla` so ownership invariants are preserved under
ring operations.

## Liveness notes

The current spec checks **safety** only (nothing bad happens). It does not yet
assert **liveness** (something good eventually happens), e.g. "a submitted Tx
frame is eventually released". Liveness/fairness will be considered alongside
`QueueLifecycle.tla`, where progress is meaningful.
