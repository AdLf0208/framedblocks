# Validation

## Pin `0a875da` — 0.2.2, the camo is a block state (2026-11-21)

Launcher-built game at the campaign pin (`ports/tools/launcher-game.sh run framedblocks`).

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings |
| `cargo test --offline` | **26 passed**, 0 failed (8 new `camo_state` tests; the red run was 19 passed / 6 failed) |
| `cargo horizon-mod build` twice after `rm -f Cargo.lock` | 36,119 B, sha256 `aea0763a…7983` (24 entries), `cmp`-identical |
| `cargo horizon-mod check framedblocks.hmod` | `framedblocks 0.2.2 (native-mod, format native-1/0a875da…/e59da909…, for 26.3)`, exit 0 |
| `cargo horizon-mod fits framedblocks.hmod --build 0a875da` | `0a875da  made for`, exit 0 |
| `cargo horizon-mod fits framedblocks.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 |
| `verify-sdk-package.py framedblocks` | 1/1 |
| e2e main | **PASS**, `EXIT:0` — phase 7 stores `minecraft:grass_block[snowy=true]`, reads it back, draws 20,870 snow px against 153 for `snowy=false`, refuses `minecraft:stone[nope=1]` without touching the stored state, and the break drops `minecraft:grass_block` |
| e2e control (same Nightly, no package) | `EXIT:1` at phase 1 |
| e2e phase 8 (recipes) | the craft grid gives `framed_cube 4` then `framed_hammer 1`, with planks 4→0, sticks 4→0 (and 0), frames 4→2 |
| `SELFTEST=1` / `SELFTEST_STATE=1` / `SELFTEST_RECIPE=1` | **FAIL**, `EXIT:1` each — the proof the carried, the state and the recipe assertions can fail |
| `check-e2e-logs.py framedblocks-e2e --strict` | `logs=7 ok=7`, exit 0 |

Evidence: `../_shared/evidence/framedblocks/sdk-0a875da/` and
`../_shared/evidence/framedblocks-e2e/e2e-*-0.2.2.log`. The slice, its red proof and its gap line
are in [notes.md](notes.md#slice-022--the-camo-is-a-block-state-2026-11-21).

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
