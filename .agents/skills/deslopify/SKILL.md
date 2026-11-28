---
name: deslopify
description: Run a read-only simplification survey of Logforth crates, pipelines, layouts, appenders, and tests, or implement findings a maintainer selected from a previous round.
---

# Deslopify

## Survey

Default to a read-only survey for simplification requests (including "look for another round") without modifying the working tree or creating report files. Inspect the latest upstream `main` commit (fetching when remote access is available) or the caller-specified target, reading the target tree via git without switching branches when it differs from the current checkout, and record the exact inspected short SHA. Check recent merges, relevant open PRs, and any review decisions the user supplied before reviving a proposal. Look beyond the last round's crates when the evidence points elsewhere.

Step back from the incremental commit history that produced the code and read the workspace as a whole against `DESIGN.md` and `AGENTS.md`. Incremental changes often leave behind patchwork states, defensive glue, or premature helpers that only look awkward from a global view. Aim for a cleaner mental model and less accidental complexity: both targeted cleanups and deeper API or boundary redesigns are in scope when they remove real friction or reveal a stronger underlying invariant. Unify code around shared contracts rather than superficial likeness, trace each candidate through real callers and `tests-integration`, and skip purely cosmetic churn.

Useful starting lenses (not an exhaustive checklist):

- Look for cleaner data representations, borrowed-vs-owned projections, or trait boundaries across `Logger`, `Record` / `RecordOwned`, `kv`, and the five extension traits (`Filter`, `Diagnostic`, `Layout`, `Append`, and `ErrorSink`) that dissolve glue code or edge cases at the root.
- Trace hot-path and concurrency invariants: build-time `Level` bitmask pruning, zero-allocation synchronous borrows and `'static` provenance, append-only `Layout::format` buffer reuse without framing leaks, running user `Display` / `Debug` formatting and destructors outside appender I/O locks, `LocalKey::try_with` teardown safety, and non-panicking operational error flow into `ErrorSink` without recursive logging loops.
- Read public APIs from the caller's perspective across `logforth-core`, peripheral `0.x` crates, and the `logforth` facade, checking constructor and builder consistency, feature-flag orthogonality, structured error context, and SemVer impact.
- In `tests-integration` and doctests, verify that fixtures stay minimal and each test guards a realistic observable regression rather than mirroring literals, obvious control flow, static type/derive guarantees, or non-contract `Debug` wording.

Open the report in the conversation with the inspected short SHA, scope, and any round-specific focus or constraints. Rank the worthwhile findings, separating straightforward internal simplifications from broader design or public API proposals. For each, give the current code and call sites, concrete maintenance or runtime cost, proposed simplification or redesign, any public API or behavior impact, and the behavior or test that distinguishes a correct change. Cite exact file paths, line ranges, symbols, and relevant PRs or commits. Surface suspected correctness bugs separately and label what has been verified. An empty or short list is a valid result. End the survey after reporting it.

## Implement selected findings

When the maintainer selects items for a PR, carry those items and their review constraints forward. Use one coherent, reviewable commit per item in one PR by default, keeping iterative trial-and-error out of commit history. Follow the repository's `cargo x` workflow (`cargo x check`, `cargo x test`, and `cargo x lint`) and `AGENTS.md` guidelines, updating `CHANGELOG.md` only for user-visible changes relative to the latest release tag. Confirm any bug fix with a focused regression test in `tests-integration`, and return the PR, commit list, and validation results for review.
