# Refactor & cleanup backlog

Opportunities to improve structure, safety, and maintainability. Update this file as items land or become obsolete.

---

---

## Config / parallelism

- Parallel thresholds live in **`src/config/parallel.rs`** (`PARALLEL` struct). When adding new rayon paths, prefer new fields there instead of crate-local `const` thresholds.

---

## Broader refactors (larger than a PR title)

- **Right pane / viewer pipeline**: `right.rs` coordinates width, wrap, and cache; worth a short design note in code or `docs/` if new viewer modes multiply.
- **KV / CSV pipeline** (`render/kv_tables/`): several submodules; keep public surface small and document which path is hot (per-frame vs cache).

---

## Tooling

- **`cargo clippy -- -W clippy::pedantic`** — noisy but useful for one-off passes on specific crates/modules.
- **`cargo udeps` / unused deps** — if the project adds a lot of optional features.

---

## Changelog (optional)

When you complete a refactor, add a one-liner under a dated heading or remove the bullet from above.
