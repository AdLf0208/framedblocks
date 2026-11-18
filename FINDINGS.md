# FINDINGS — can FramedBlocks' camo be expressed at pin `c9d474d`?

Question (the brief's probe): a framed block stores **another block's
appearance** and the client draws that block in the frame's place. Does the pin
have the three pieces — a block entity on a plain content block, a way to sync
its data to the client, and a client render hook that draws an arbitrary
block's model at that position — or is this port blocked?

**Answer: feasible. It is implemented in this port; nothing was faked.** There
is one pin limitation, found in game and recorded below: the block entity
needs a template that already owns one.

## 1. Block entity — `native_block_entity` (API 15)

- Hook point in the **pin export**
  (`target/mixins/original/32051c454db029bf4671`, revision `c9d474d`):
  `crates/horizon-client/src/game/native_hooks.rs:1555`
  (`Game::native_block_entity(&mut self, block, entity)`).
- The engine calls it from `Game::new_block_entity`
  (`native_hooks.rs:4200-4212`), on the server when a block is placed and on
  the client when a block change arrives. It wraps the answer in
  `BlockEntity::Mod` with `ModBlockEntity { id, nbt, .. }`; the compound is
  saved with the chunk and read through `ModBlockEntityFields.nbt`.
- **Pin limitation (the only one found):** `new_block_entity` accepts the mod's
  answer only when the block's **template** has a block entity:
  ```rust
  let state = self.pack.assets.registry.default_state(name)?;
  if !self.res.facts().get(state).block_entity {
      super::mods::warn_no_block_entity(name, &id);
      return None;
  }
  ```
  In game this logged
  `framedblocks:framed_cube: no block entity framedblocks:framed_cube, its template has none (copy a chest or furnace)`
  for the first content template (`minecraft:oak_planks`). API 129 ("a block
  entity whatever its template") is in a **later** Horizon revision (the live
  checkout's docs), not at this pin.
- Worked around without faking anything: the content block copies
  `minecraft:spawner` — a `BaseEntityBlock` with a block entity, **no state
  properties**, no menu and no use interaction, and a full-cube shape. The
  content declarations override hardness 2, resistance 3, wood sounds and
  `requires_tool: false`; its blockstate model is empty, so the block's own
  geometry is nothing. The mod entity is created and the camo lives in it.
  A barrel would also work but opens a menu on every right click; the spawner
  does not.

## 2. Server → client sync — `native_block_entity_update_tag` (API 45) + `native_block_entity_sync` (API 52)

- Hook point: `native_hooks.rs:1647`; it returns the compound a client's copy
  merges field by field (`ModBlockEntity::merge_update_tag`).
- Re-send on change: `native_block_entity_sync` (`native_hooks.rs:3476`)
  queues the block and its update tag as `Level.sendBlockUpdated` does.
- Read/change server-side: `native_with_mod_block_entity`
  (`native_hooks.rs:3446`) hands the live `ModBlockEntity` and marks the chunk
  saved.

## 3. Client render — `native_block_entity_submit` (API 71) + `native_draw_block`

- Hook point: `native_hooks.rs:1666`, called once a frame from the block
  entity draw loop for mod block entities whose template has no vanilla
  renderer; `classify` is keyed by the **block name**, so our mod name never
  gets a vanilla renderer kind.
- `native_draw_block(draw, block, properties)` (`native_block_entity_draw.rs:439`)
  draws **any block's baked model** in the state with `properties`, from the
  client's registry, under the block entity's pose — vanilla's
  `submitBlockModel`, exactly the "draw another block here" primitive.
- `native_draw_cube` (`native_block_entity_draw.rs:406`) draws the frame's own
  texture (the no-camo appearance) from a sprite.
- The frame's own blockstate model is **empty**: vanilla air's model
  (`/home/adlf/Desktop/horizon/.local/horizon-client/26.3/assets/minecraft/models/block/air.json`)
  is a particle texture and no elements, and the model loader accepts it. So
  the frame draws nothing by itself and the block entity renderer is the only
  geometry — no double-draw and no z-fighting.

## 4. The rest of the lifecycle

- Apply/remove: `native_use_item_on` (API 13, `native_hooks.rs:2292`) has the
  acting player's held stack and can claim the click; the acting player's
  state is swapped in, so `held_mut` works (see `hook-feasibility.md`,
  akashictome).
- Place with an off-hand camo: `native_place_block` (API 14,
  `native_hooks.rs:2419`) runs after the block and its entity exist, and the
  off-hand is in the swapped-in player state.
- Break drop: `native_destroy_block` (API 14, `native_hooks.rs:2407`) runs
  before the block entity is removed; `native_loot_drops` (API 75,
  `native_hooks.rs:2642`) adds the camo item to the stacks about to drop in
  the same tick. `NativeLootDrops` (`native_hooks.rs:673`) carries
  `block: Option<([i32; 3], BlockStateId)>`.
- Give the camo item back on removal: `native_give_stack` (API 98,
  `native_server.rs:708`) returns what did not fit, so "no room → keep the
  camo" is expressible.
- BE readout command: `native_register_commands` (`native_hooks.rs:2164`) and
  `native_perform_command` (`native_hooks.rs:2172`); the tick phase is
  `native_server_tick` (`native_hooks.rs:1273`) with `ServerTickPhase`
  (`game/native_lifecycle.rs:54`).

## Upstream behaviour read (analysis only; the port is a rewrite)

Repo `XFactHD/FramedBlocks`, branch `26.1` (MC 26.1.2, mod 11.4.0), commit
`4f74728ea3d4f051b1fc96626e22bb04a7f4f8cc`. The ATM10 pack ships 10.6.1 for
1.21.1; the camo semantics are the same.

- `src/main/java/io/github/xfacthd/framedblocks/common/block/cube/FramedCubeBlock.java:11`
  — the plain full-cube frame, default state `alt=false`, `reinforced=false`,
  `solid_bg=false`; those three flags are outside this slice.
- `api/block/IFramedBlock.java:92-107` — frame properties: wood map colour,
  `strength(2F)`, wood sounds, ignited by lava; `noOcclusion()` only when the
  type cannot occlude with a solid camo (the cube can, so it occludes).
- `api/block/blockentity/FramedBlockEntity.java:112-157` — `handleInteraction`:
  empty camo → a camo factory for the held stack; set camo → removal tool,
  camo rotation, then modifiers; else `TRY_WITH_EMPTY_HAND`.
- `api/camo/block/AbstractBlockCamoContainerFactory.java:23-38` — `applyCamo`
  reads the state as `BlockItem.getBlock().defaultBlockState()`
  (`:70-76`), refuses framed blocks (`:77-79`), and extracts one item in
  survival (`:28-37`).
- `common/data/camo/CamoContainerFactories.java:72-87` — `findCamoFactory`:
  any `BlockItem`, except a framed block's; `:89-104` `isValidRemovalTool`.
- `api/camo/TriggerRegistrar.java:12` — the default removal tool is the framed
  hammer (or an item with the wrench-empty ability).
- `api/block/blockentity/FramedBlockEntity.java:231-252` — `setCamo` /
  `clearCamo`; `:243-252` keeps the camo when the returned item does not fit.
- `api/block/IFramedBlock.java:148-152` — `tryApplyCamoImmediately`: a frame
  placed with a block item in the **off hand** takes it as the camo;
  `FramedBlockEntity.java:682` allows it for a single cube.
- `api/block/IFramedBlock.java:215-224` + `FramedBlockEntity.java:692-716` —
  a break drops the camo item beside the frame (`addCamoDrops`).
- `client/model/baked/FramedBlockStateModel.java:106-146` — the client draws
  the camo block's model in the frame's place (the frame overlay/geometry is
  the rest of upstream's model system, outside this slice).

## What this slice deliberately leaves out (recorded, not faked)

- Every other frame shape (slopes, panels, pillars, doors, double blocks,
  waterlogging, the `alt`/`reinforced`/`solid_bg` flags, glowstone,
  intangibility, overlays, the camo applicator, the wrench, camo rotation,
  state cycling, the item's camo preview).
- A camo is a **default-state** block camo (upstream's plain block-item path).
  A state-specific camo (stairs facing, a rotated log axis) is not expressible
  from an item at this pin; upstream stores the placed state, the port stores
  the block id and draws its default state.
- Only a **player break** returns the camo (the hook pair above). An
  explosion, a mob or a piston break does not roll `destroy_block`; the frame's
  own loot table still drops the frame, the camo is lost. Upstream returns the
  camo for any break.
- The frame's texture is drawn only when no camo is set; upstream also draws
  the frame overlay around a camo.
- Sounds/particles on apply/remove are not played.
- The mod block entity rides a **spawner template** because the pin requires a
  template with one (API 129 is later); a spawner's own mob-spawning behaviour
  does not apply to a `BlockEntity::Mod`.
