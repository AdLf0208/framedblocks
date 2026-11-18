# FramedBlocks — a Horizon port

**FramedBlocks** is a Minecraft mod. Fancy building blocks. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [XFactHD/FramedBlocks](https://github.com/XFactHD/FramedBlocks) — licence **LGPL-3.0-only**.

| | |
|---|---|
| Upstream mod | FramedBlocks ([source](https://github.com/XFactHD/FramedBlocks)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `framedblocks.hmod` (install it with the Horizon launcher) |
| Licence | **LGPL-3.0-only** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

### Not in this slice

Every other frame shape, double blocks, waterlogging, the frame flags,
glowstone/intangible/reinforced/emissive modifiers, overlays, the camo
applicator, the wrench, camo rotation, state cycling, the item's camo preview,
state-specific camos, and camo returns for non-player breaks. The full list is
in [notes.md](notes.md#deviations-and-gaps); the pin probe is in
[FINDINGS.md](FINDINGS.md).

## Not yet ported

- **One frame shape only** (the full cube). Slopes, panels, pillars, doors,
- **Default-state block camos.** Upstream stores the placed `BlockState`; a
- **The camo returns on a player break only.** `on_destroyed_by_player` is
- **No frame overlay around a camo.** The frame texture is drawn only when no
- **No apply/remove sound or particle.**
- **A camo block with its own block entity** (a chest) is drawn as its baked
- The item model does not preview a camo (no item component in this slice).
- **The camo returns only into an empty main-inventory slot.** Upstream merges

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `framedblocks.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

FramedBlocks was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **LGPL-3.0-only**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/XFactHD/FramedBlocks

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run framedblocks` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/framedblocks/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
