# Development Setup

This page covers the local toolchain for working on ZeroGate, including the
formal-methods tooling needed to run the TLA+ model checks.

## Rust

ZeroGate builds on the **nightly** toolchain (CI uses `dtolnay/rust-toolchain@nightly`).

```bash
rustup toolchain install nightly
rustup component add rustfmt clippy --toolchain nightly
```

Standard checks:

```bash
cargo +nightly fmt --all -- --check
cargo +nightly clippy --workspace --all-targets -- -D warnings
cargo +nightly test --workspace
./scripts/audit_no_unsafe.sh
```

## Formal methods: Java + TLA+ (TLC)

The TLA+ specs under [`../formal/tla/`](../formal/tla/) are checked with **TLC**,
which ships in `tla2tools.jar` and runs on a JVM. The runner is
[`../scripts/run_tla.sh`](../scripts/run_tla.sh).

### Requirements

| Tool | Version | Notes |
|------|---------|-------|
| Java (JRE/JDK) | **Temurin JDK 21** (any Java 11+ works) | provides the JVM for TLC |
| `tla2tools.jar` | 1.7.4 (matches CI) | the TLC model checker |

### One-time local setup

1. **Install Java.** Temurin JDK 21 is recommended for parity with CI.
   - macOS: `brew install --cask temurin@21`
   - Debian/Ubuntu: install a Temurin/OpenJDK 21 package
   - Windows: `choco install temurin` (or the Adoptium installer)

   Verify: `java -version`.

2. **Get `tla2tools.jar`.** Download once from the TLA+ releases page:
   <https://github.com/tlaplus/tlaplus/releases> (v1.7.4 to match CI).

3. **Point the runner at it.** The preferred mechanism is the **`TLA2TOOLS_JAR`**
   environment variable:

   ```bash
   export TLA2TOOLS_JAR=/path/to/tla2tools.jar
   ```

   Alternatively, place the jar at one of the auto-searched locations (e.g. the
   repo root `tla2tools.jar`, `~/tla2tools.jar`, `~/.tla/tla2tools.jar`,
   `/usr/local/lib/tla2tools.jar`, `/opt/tlaplus/tla2tools.jar`). The legacy
   `TLA_TOOLS_JAR` variable is still honored for back-compat, but `TLA2TOOLS_JAR`
   is preferred.

> The runner **never downloads** the jar itself (so it stays deterministic and
> offline-friendly) and **never fakes success** — if Java or the jar is missing
> it exits non-zero with instructions.

### Running the checks

```bash
./scripts/run_tla.sh quick            # N=3 and N=4 (what CI runs)
./scripts/run_tla.sh extended         # N=3, N=4, N=5
./scripts/run_tla.sh frame-ownership  # default committed cfg (N=3)
./scripts/run_tla.sh --help
```

`N>=8` / symmetry-reduced checking is not implemented yet; the runner says so
explicitly rather than pretending. See [`../specs/README.md`](../specs/README.md).

## How CI obtains the tooling

Both CI systems install a JDK and download `tla2tools.jar` at job time, then run
`./scripts/run_tla.sh quick` — neither vendors the jar into the repo:

- **GitHub Actions** ([`../.github/workflows/tla.yml`](../.github/workflows/tla.yml)):
  `actions/setup-java` (Temurin 21) + `curl` of the pinned `tla2tools.jar`, with
  `TLA2TOOLS_JAR` exported to the runner temp path. Triggers on `pull_request`
  (any base, so stacked PRs are covered), `workflow_dispatch` (with a `mode`
  input), and `push` to `main`.
- **GitLab CI** (`tla_model_check` job, stage `formal`, in
  [`../.gitlab-ci.yml`](../.gitlab-ci.yml)): `eclipse-temurin:21-jdk` image +
  `curl` of the pinned jar into `TLA2TOOLS_JAR`.

`tla2tools.jar` is intentionally **not vendored**; it is downloaded per job and
pointed to via `TLA2TOOLS_JAR`.
