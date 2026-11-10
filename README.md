# framedblocks (Horizon port)

A Horizon port of [framedblocks](XFactHD/FramedBlocks) — LGPL-3.0-only.

**Status: first slices.** What is below is what the port actually does today.

## Implemented so far

- Repo `XFactHD/FramedBlocks`, branch `26.1`, commit `4f74728e`.
- `src/main/java/io/github/xfacthd/framedblocks/common/block/cube/FramedCubeBlock.java:11`
- — the full-cube frame; default state `alt=false`, `reinforced=false`,
- `solid_bg=false` (those three flags are outside this slice).
- `api/block/IFramedBlock.java:92-107` — frame properties: wood map colour,
- `strength(2F)`, wood sounds, ignited by lava, occluding for a solid camo.
- `api/block/blockentity/FramedBlockEntity.java:112-157` — `handleInteraction`:
- an empty camo takes a camo factory for the held stack; a set camo takes the

## Not yet ported

- One frame shape only** (the full cube). Slopes, panels, pillars, doors,
- double blocks, waterlogging, `alt`/`reinforced`/`solid_bg`, glowstone,
- intangibility, overlays, the camo applicator, wrench rotation, camo
- rotation, state cycling and the item's camo preview are not ported.
- Default-state block camos.** Upstream stores the placed `BlockState`; a
- block item gives its block's default state, and the port stores the block
- id and draws that default state. A state-specific camo (a stair's facing, a
- log's axis) is not expressible from an item at this pin.

## Install

Build the bundle with a native-mod capable launcher, or add the file from the
registry once it is listed. Pinned to Horizon `1d63e0a` and horizon-mixins
`5a64af93` for game 26.3.

## Build from source

```sh
cargo test
python3 tools/horizon.py check --horizon <horizon checkout>
python3 tools/horizon.py build --horizon <horizon checkout>
python3 tools/package.py
```

## Credits and licence

Original mod: XFactHD/FramedBlocks. Port: AdLf0208. Licence: LGPL-3.0-only — see [LICENSE](LICENSE);
the original notices are preserved.
