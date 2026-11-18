# FramedBlocks for Horizon — framed cube + block camo

A native Horizon port of [FramedBlocks](https://github.com/XFactHD/FramedBlocks)
(LGPL-3.0-only). This slice is one **framed cube** whose camo is a real block:
the block entity stores another block's id, the client draws that block's model
in the frame's place, a right click with a block item applies it, the framed
hammer removes it, and a player break returns it.

Upstream was read for behaviour only; this is a clean-room Rust rewrite. The
port is **local-only** until the licence allows derivatives and the maintainer
approves publishing (ARCHITECTURE.md §8).

## What it does

- `framedblocks:framed_cube` — a full-cube frame on the **spawner** template
  (chosen when the retired bundle's pin only gave a mod block entity to a block
  whose template owns one; at the SDK the frame's own block entity type is
  declared in code, so the template is legacy). Its own blockstate model is
  empty; the block entity renderer draws either the frame texture or the camo
  block's default-state model.
- Right click with any block item → that block becomes the camo (one item in
  survival, none in creative).
- Right click with `framedblocks:framed_hammer` → the camo comes off and its
  item goes back to the player (kept when there is no room).
- Place a frame with a block item in the off hand → it becomes the camo.
- Break the frame → the frame and the camo item drop.
- The camo is in the block entity's saved compound, so it survives save and
  reopen.
- `/framedblocks camo <x> <y> <z>` reads that compound back from the server:
  the camo block and the item, or `none` (the e2e's storage check).

Recipes: the upstream frame (planks + sticks → 4) and hammer (frame + sticks).

## Not in this slice

Every other frame shape, double blocks, waterlogging, the frame flags,
glowstone/intangible/reinforced/emissive modifiers, overlays, the camo
applicator, the wrench, camo rotation, state cycling, the item's camo preview,
state-specific camos, and camo returns for non-player breaks. The full list is
in [notes.md](notes.md#deviations-and-gaps); the pin probe is in
[FINDINGS.md](FINDINGS.md).

## Build and check

This is a flat SDK package (mod API 0.2): `horizon-mod.toml` + `Cargo.toml` + `src/lib.rs` +
`src/runtime.rs` + `assets/` + `data/`. Every command runs directly in the port; a build slot
only provides the lease and `CARGO_INCREMENTAL=0`.

```sh
ports/tools/build-slot.sh run --job sdk-fb --port ports/framedblocks --step test \
    -- bash -c 'cd ports/framedblocks && cargo test --offline'
ports/tools/build-slot.sh run --job sdk-fb --port ports/framedblocks --step lint \
    -- bash -c 'cd ports/framedblocks && cargo fmt --check &&
        cargo clippy --all-targets --offline -- -D warnings'
rm -f Cargo.lock
ports/tools/build-slot.sh run --job sdk-fb --port ports/framedblocks --step build \
    -- bash -c 'cd ports/framedblocks && cargo horizon-mod build &&
        cp target/horizon-mod/framedblocks.hmod ./framedblocks.hmod'
ports/tools/build-slot.sh run --job sdk-fb --port ports/framedblocks --step check \
    -- bash -c 'cd ports/framedblocks && cargo horizon-mod check framedblocks.hmod &&
        cargo horizon-mod fits framedblocks.hmod --build nightly'
```

`cargo horizon-mod` fetches the pinned SDK into `.horizon/sdk/` (gitignored, never packed);
`build` packs the `.hmod` and does not compile against the game — that is
`horizon-launcher native-build ports/framedblocks --instance <i> --trust-native-code`.

The e2e runs the launcher-built game `--offscreen` (screenshots need a renderer) in a
scratch home. It checks the frame texture, the camo drawn in its place, the drops, an
uncamo'd control, and a save/reopen. `COUNTERFACTUAL=1` repeats it with a diamond camo and
requires diamond, not gold. It was last run on the retired `.hnmod` bundle; the SDK
migration did not re-run it (see [notes.md](notes.md#migration-to-the-sdk-format-2026-11-18-sdk-0b14f8a)).
