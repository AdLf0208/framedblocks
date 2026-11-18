# Validation

> **Superseded (2026-11-18):** the current ladder and package evidence are in
> [`notes.md`](notes.md#migration-to-the-sdk-format-2026-11-18-sdk-0b14f8a) and
> `../_shared/evidence/framedblocks/sdk-0b14f8a/`. Everything below is the history of the
> retired `.hnmod` bundle (pins `c9d474d`/`1d63e0a5`), whose staged ladder no longer exists.

## Campaign pin `1d63e0a5` (2026-11-10)

Migrated from `c9d474dd` / mixins `d9c83d7` to Horizon
`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` (mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`,
schema 6), through `ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0` and
the pin-matched shared `horizon-ctl`:

| Check | Result |
|---|---|
| `cargo fmt --check` + `rustfmt --edition 2024 --check mixins/framedblocks/*.rs` | pass |
| `cargo test --locked --offline` | 12 passed, 0 failed |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | pass, 0 warnings |
| `tools/horizon.py stage/check/test/build --horizon …/pins/horizon-1d63e0a` | `cargo_exit: 0`, 0 warnings, 12 embedded tests, fast build |
| `EMBEDDED=1 tools/e2e.sh` (control + camo + save/reopen; one `.hnmod`, no `--dev-mod`) | **PASS**, `EXIT:0` |
| `EMBEDDED=1 COUNTERFACTUAL=1 tools/e2e.sh` | **COUNTERFACTUAL PASS**, `EXIT:0` |
| `content/framedblocks.hmod` | 7,612 B, SHA-256 `6fd279ee…1960d`, 16 entries |
| `framedblocks.hnmod` (content embedded) | 19,799 B, SHA-256 `008508db…430e`, 16 entries |

The control leg runs the same binary with `.horizon-cache` made unusable: no
framedblocks item appears and the block stays `minecraft:air`, so the ids proved
in the main leg come from the embedded content file (written byte-identical to
`content/framedblocks.hmod`).

## Pin `c9d474dd` (2026-11-05, historical)

Validated for `c9d474d` (`nightly-source/2026-11-01-c9d474d`), mixins `d9c83d7`,
using the real bundle
and the prepared 26.3 resources at
`/home/adlf/Desktop/horizon/.local/horizon-client/26.3`. All builds used
`CARGO_INCREMENTAL=0` and `CARGO_BUILD_JOBS=2`.

| Check | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo test --workspace --locked --offline` | 12 passed, 0 failed |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | pass, 0 warnings |
| `python3 tools/horizon.py stage` | pass, `stage_changed: true`, 184 s |
| `python3 tools/horizon.py check` | pass, `cargo_exit: 0`, 0 warnings |
| `python3 tools/horizon.py test` | pass, `cargo_exit: 0`, `game::framedblocks::tests` 12 passed, 0 warnings |
| `python3 tools/horizon.py build` | pass, `cargo_exit: 0`, `target/horizon-cargo/fast/horizon-client` |
| ctl build (`--bin horizon-ctl`, fast) | pass, `target/horizon-cargo/fast/horizon-ctl` |
| `tools/e2e.sh` (offscreen renderer) | **PASS**, exit 0, `/tmp/opencode/framedblocks-e2e-final.log` |
| `COUNTERFACTUAL=1 tools/e2e.sh` | **COUNTERFACTUAL PASS**, exit 0, diamond not gold |
| `cargo horizon-mod build` / `check` content | 7,612 bytes, SHA-256 `6fd279ee…1960d`, "no code: content only" |

The e2e proves the camo end to end: the frame texture is drawn by the block
entity renderer (66,059 brown pixels), a right click with a gold block stores
the camo in the block entity (`/framedblocks camo` reads it) and the client
draws gold (54,468 pixels); the break returns frame + camo; the framed hammer
removes and returns the camo; an uncamo'd frame drops no camo; and the camo is
still stored and drawn after save/quit/reopen (67,495 gold pixels).

The counterfactual repeats the identical scenario with a diamond block: the
block entity and the screen show diamond, not gold (57,094 cyan pixels), so a
renderer that ignored the stored camo or one hard-coded to gold fails.

These are compile/test/interactive checks at this pin. The port is local-only
until the licence allows derivatives and the maintainer approves publishing
(ARCHITECTURE.md §8).
