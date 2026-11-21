# FramedBlocks — Horizon port (slice: framed cube + block camo)

> **Format (2026-11-18): the port is now a flat SDK `.hmod`.** Horizon retired `.hnmod` in the
> mod-API merge (#16941, launcher ≥ 0.2.70); the bundle this file described (PR **#820**, closed in
> the 2026-11-18 format purge) was migrated to `horizon-mod.toml` + `Cargo.toml` +
> `src/lib.rs` (`impl mod_api::Hooks`) + `assets/`/`data/` at Horizon
> **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** / horizon-mixins
> **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, mod API `0.2`, port version **0.2.0**.
> Read **`## Migration to the SDK format`** first: it is the current state — file map,
> helper→hook table, build/check evidence, gaps and the changelog. Everything above it is the
> history of the retired bundle (pin `1d63e0a5`, `.hnmod`, `mixins/`, the staging ladder) and is
> kept as the record, not as instructions — none of `tools/horizon.py`, `compatibility.json`,
> `mixins/<id>/` or the `.hnmod` exists any more. `src/runtime.rs` was **byte-identical** to the
> retired `mixins/framedblocks/runtime.rs` (sha256 `26616be4…d525f`) from 0.2.0 to 0.2.1; 0.2.2
> renames one variant (`Held::Block` → `Held::BlockItem`, sha256 `d034d5da…8abd`) because the
> `fits` checker reads a bare `Block` as the SDK's non-exhaustive `type:Block` draft struct — see
> [Slice 0.2.2](#slice-022--the-camo-is-a-block-state-2026-11-21).


Ported from **FramedBlocks** (`XFactHD/FramedBlocks`, **LGPL-3.0-only**).
Upstream analysed at branch `26.1` (MC 26.1.2, mod 11.4.0), commit
`4f74728ea3d4f051b1fc96626e22bb04a7f4f8cc`; the ATM10 pack ships 10.6.1 for
1.21.1, whose camo semantics are the same. Analysis only: the port is a
clean-room Rust rewrite, not a translation. **Local-only** until the licence
allows derivatives *and* the maintainer approves publishing (ARCHITECTURE.md
§8).

This slice ships **one full-cube frame whose camo is a real block**: a framed
cube stores another block in its block entity, the client draws that block's
model in the frame's place, a right click with a block item applies it, the
framed hammer removes it, and a player break returns it. There is no fake
camo: the appearance is the camo block's own baked model.

## Probe result (brief step 1)

**Feasible at pin `c9d474d`.** The three needed capabilities exist and were
verified against the pin's source before any port code was written:

| Need | Pin API | Evidence |
|---|---|---|
| block entity (template must own one) | `native_block_entity` (API 15) | pin `native_hooks.rs:1555`, `new_block_entity` `:4200` |
| data to clients | `native_block_entity_update_tag` (45) + `native_block_entity_sync` (52) | pin `native_hooks.rs:1647`, `:3476` |
| draw another block at a position | `native_block_entity_submit` (71) + `native_draw_block` | pin `native_hooks.rs:1666`, `native_block_entity_draw.rs:439` |

The pin's `new_block_entity` refuses a mod block entity on a template with none
("no block entity …, its template has none"); API 129 ("whatever its
template") is a later revision. The port uses `minecraft:spawner` (a
`BaseEntityBlock` with no properties, menu or use). See FINDINGS.md §1.

The full probe with file:line and the upstream behaviour map is in
[`FINDINGS.md`](FINDINGS.md).

## Upstream behaviour extracted (files read)

Repo `XFactHD/FramedBlocks`, branch `26.1`, commit `4f74728e`.

- `src/main/java/io/github/xfacthd/framedblocks/common/block/cube/FramedCubeBlock.java:11`
  — the full-cube frame; default state `alt=false`, `reinforced=false`,
  `solid_bg=false` (those three flags are outside this slice).
- `api/block/IFramedBlock.java:92-107` — frame properties: wood map colour,
  `strength(2F)`, wood sounds, ignited by lava, occluding for a solid camo.
- `api/block/blockentity/FramedBlockEntity.java:112-157` — `handleInteraction`:
  an empty camo takes a camo factory for the held stack; a set camo takes the
  removal tool; otherwise the click falls through.
- `api/camo/block/AbstractBlockCamoContainerFactory.java:23-38,70-79` —
  `applyCamo`: the camo state is `BlockItem.getBlock().defaultBlockState()`;
  a framed block is refused; survival consumes one item.
- `common/data/camo/CamoContainerFactories.java:72-104` — `findCamoFactory`
  (any block item except a framed block's) and `isValidRemovalTool`.
- `api/camo/TriggerRegistrar.java:12` — the default removal tool is the
  framed hammer (or an item with the wrench-empty ability).
- `api/block/blockentity/FramedBlockEntity.java:231-252` — `setCamo` /
  `clearCamo`; a removal keeps the camo when the returned item does not fit.
- `api/block/IFramedBlock.java:148-152`, `FramedBlockEntity.java:682` — a
  frame placed with a block item in the **off hand** takes it as its camo.
- `api/block/IFramedBlock.java:215-224`, `FramedBlockEntity.java:692-716` — a
  break drops the camo item beside the frame.
- `client/model/baked/FramedBlockStateModel.java:106-146` — the client draws
  the camo block's model in the frame's place.

Where the rewrite differs from upstream is listed under **Deviations** below.

## Layout

```text
ports/framedblocks/
  horizon-mod.toml            the only manifest (id, version 0.2.0, api 0.2, [native] pins)
  Cargo.toml                  one crate; the only dependency is horizon-mod-api from .horizon/sdk/
  src/lib.rs                  the one #[mod_api::horizon_mod] impl Hooks, the two Parts, the glue
  src/runtime.rs              pure camo decision model + 12 unit tests (byte-identical, carried)
  assets/framedblocks/        blockstate, item definitions, models, both textures, lang
  data/framedblocks/          block + item facts, loot table, two recipes
  data/minecraft/             the mineable/axe tag
  LICENSE, LICENSE-NOTICE.md  both travel inside the package (LGPL-3.0-only)
  framedblocks.hmod           the delivered package (port root)
  tools/e2e.sh                screenshot differential + drops + save/reopen (needs a launcher build)
  notes.md, FINDINGS.md, VALIDATION.md
```

The retired `mixins/`, `content/`, `compatibility.json`, `rust-toolchain.toml`, `tools/horizon.py`,
`tools/package.py`, the `.hnmod` and the content `.hmod` are under
[`ports/_shared/evidence/framedblocks/retired-hnmod-0.1.0/`](../_shared/evidence/framedblocks/retired-hnmod-0.1.0/).

## Runtime (`runtime.rs`)

The pure model is upstream's interaction decision:

- `Held::{Block, Hammer, Other}`, `CamoAction::{Apply, Remove, None}`,
  `Render::{Camo, Frame}`.
- `camo_action`: a block item on an empty frame applies its block; the hammer
  on a set camo removes it; a block item on a set camo does nothing; a framed
  block and air are never camos.
- `render`: a set camo draws the camo block's default-state model, otherwise
  the frame's own texture.
- `camo_drop`, `consumes` (survival consumes, creative does not).

12 unit tests cover that matrix, including the empty-string camo a removal
writes to clear a client's copy (found by the test: `is_camo_block("")` first
answered true, which made a removed camo draw `Camo("")`).

## Content (inside the flat package)

- Block `framedblocks:framed_cube` on the **`minecraft:spawner`** template
  (hardness 2, resistance 3, wood sounds, no tool requirement; upstream's
  `strength(2F)` + wood). Its blockstate model is **empty** (a particle
  texture only, like vanilla's air model), so the block's own geometry is
  nothing and the block entity renderer is the only source of pixels.
  The spawner template was chosen at `1d63e0a5` because that pin only gave a mod
  block entity to a block whose template owns one; at the SDK the frame's own
  block entity type is declared in `src/lib.rs` (`block_entity_type` +
  `block_entity_part`), so the template is no longer load-bearing. It is carried
  unchanged because no in-game run at `0b14f8a` could confirm a plain template
  (`minecraft:oak_planks`, the pattern `justdirethings` uses) — the next in-game
  slice should try it and record the result.
- Item `framedblocks:framed_hammer` (`minecraft:stick` template, stack 1); the
  frame block has its own item (the content format adds one by default).
- Assets carried from the upstream repo (LGPL, local-only):
  `textures/block/framed_block.png`, `textures/item/framed_hammer.png`; the
  item model is a `cube_all` of the frame texture (upstream's item model shows
  the camo of the item's component, which this slice does not have).
- Recipes and loot are upstream's (`framed_cube`: planks + sticks → 4;
  `framed_hammer`: frame + sticks), the block is in `minecraft:mineable/axe`,
  and the loot table drops the frame; the camo is added by the native drop
  hook.

## Game glue (`src/lib.rs`)

- `BlockEntityPart::block_entity` gives the frame an empty saved compound on
  both sides; the frame's block entity type is declared for
  `framedblocks:framed_cube` in `register`.
- The camo compound is `Camo` (block id) + `CamoItem` (the item a break gives
  back). A removal writes **empty strings**, not absent keys: a client merges
  an update tag field by field, so an absent key would leave the old camo on
  screen.
- `block_entity_submit` draws the camo with `draw.block`; when no camo is set,
  or the camo block is not in this client's resources, it draws the frame with
  `draw.cube` and the frame sprite.
- A right click with a block item writes the camo through
  `BlockEntityRef::merge_data` (the `/data merge block` path, which also marks
  it changed and syncs it), consumes one in survival with
  `LivingRef::set_hand_item`, and answers `InteractionResult::SuccessServer`.
- The hammer puts the camo item into the first free main-inventory slot
  (`PlayerRef::set_inventory_slot`) and only clears the camo when it went in;
  with no room the camo stays, as upstream's `removeCamo`.
- A frame placed with a block item in the off hand takes it as its camo in the
  frame item's `ItemPart::on_block_placed_by` (upstream
  `tryApplyCamoImmediately`), consuming one in survival.
- A player break reads the camo in `BlockPart::on_destroyed_by_player`, while
  the block entity still exists, and `loot_drops` adds the item to the same
  tick's roll. The tick's end forgets a break whose loot never rolled
  (creative). This is the retired bundle's shape because `loot_drops` is the
  only drop hook with a full `ServerHost` (it can resolve an `ItemKey` for an
  arbitrary camo item); `block_drops` provides the block entity but only a
  `LevelHost`, which has no item-key lookup.

## Deviations and gaps

1. **One frame shape only** (the full cube). Slopes, panels, pillars, doors,
   double blocks, waterlogging, `alt`/`reinforced`/`solid_bg`, glowstone,
   intangibility, overlays, the camo applicator, wrench rotation, camo
   rotation, state cycling and the item's camo preview are not ported.
2. ~~**Default-state block camos.**~~ **Superseded by 0.2.2**: the camo is a
   block **state** (`minecraft:grass_block[snowy=true]`), drawn with its
   properties; a *block item* still gives its block's default state (there is no
   item that carries a state), and the wrench/applicator that would rotate one
   are not ported, so a non-default state is set by
   `/framedblocks camo <x> <y> <z> <state>` (a command or a blueprint).
3. **The camo returns on a player break only.** `on_destroyed_by_player` is
   player-only; an explosion, mob or piston break still drops the frame from
   its loot table but loses the camo. Upstream returns the camo for any break.
   (At the SDK `block_drops` would cover every break, but it hands over only a
   `LevelHost`, which cannot resolve an arbitrary camo item to an `ItemKey`.)
4. **No frame overlay around a camo.** The frame texture is drawn only when no
   camo is set; upstream draws a frame border over the camo too.
5. **No apply/remove sound or particle.**
6. **A camo block with its own block entity** (a chest) is drawn as its baked
   block model, without the block entity animation (`draw.block` draws the
   block model only).
7. The item model does not preview a camo (no item component in this slice).
8. **The camo returns only into an empty main-inventory slot.** Upstream merges
   it into a partial matching stack too; with no empty slot the port keeps the
   camo even if a partial stack would have taken it (see the migration section,
   behaviour change 2).
9. **`/framedblocks camo` is registered by the SDK's command tree** and answers
   through `Commands::feedback`. The line is the same; `parse_camo_command` is
   kept and tested but the live path uses typed arguments, so a malformed line
   never reaches the mod (the dispatcher refuses it first).

Skills: superpowers/using-superpowers,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

## Verification

Upstream analysed: `XFactHD/FramedBlocks`, branch `26.1`, commit
`4f74728ea3d4f051b1fc96626e22bb04a7f4f8cc` (MC 26.1.2, mod 11.4.0). The clone
is analysis-only and was deleted after this section was written.

### Runtime tests

| Run | Command | Result |
|---|---|---|
| standalone | `cargo test --workspace --locked --offline` | 12 passed, 0 failed |
| embedded (pin) | `horizon.py test` (`game::framedblocks::tests`) | 12 passed, 0 failed, 531 filtered |

The test matrix covers apply/remove/none, the framed-block and air refusals,
occupied-frame block items, render choice, the drop, survival vs creative
consumption, and the `/framedblocks camo` parser. Mutation check: changing the
command root name made `the_camo_command_takes_three_integers` fail; restored.

### Campaign-pin migration `1d63e0a5` (2026-11-10)

`compatibility.json` and the mixin plan now name Horizon
`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` (`game.rs` SHA-256 `64813cf4…cef80f`)
/ horizon-mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`; `tools/horizon.py`
is the campaign template's wrapper (`--lib game::framedblocks::`). The built
content mod is carried inside the one delivered `.hnmod` at
`mixins/framedblocks/data/framedblocks-content.hmod` and the `embedded-content`
injection at `content_root` writes it to `.horizon-cache/` (the AE2 one-file
pattern), so the e2e launches with no `--dev-mod`.

| Check | Result |
|---|---|
| `cargo fmt --all --check` + `rustfmt --edition 2024 --check mixins/framedblocks/*.rs` | clean |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, **0 warnings** |
| `cargo test --locked --offline` | **12 passed**, 0 failed |
| `tools/horizon.py stage/check/test/build --horizon …/pins/horizon-1d63e0a` | `cargo_exit: 0`, **0 warnings**, 12 embedded tests, fast build |
| `EMBEDDED=1 tools/e2e.sh` (control + camo + save/reopen) | **PASS**, `EXIT:0` — `ports/_shared/evidence/framedblocks/e2e.log` |
| control leg, same binary, `.horizon-cache` unusable | no framedblocks item, the block stayed `minecraft:air` — so the ids come from the embedded file |
| embedded file | written 7,612 B, byte-identical to `content/framedblocks.hmod` |
| `EMBEDDED=1 COUNTERFACTUAL=1 tools/e2e.sh` | **COUNTERFACTUAL PASS**, `EXIT:0` (diamond, not gold) — `e2e-counterfactual.log` |
| `content/framedblocks.hmod` | **7,612 B**, SHA-256 `6fd279ee4497…1960d`, 16 entries, content only |
| `framedblocks.hnmod` | **19,799 B**, SHA-256 `008508dbf22d…430e`, 16 entries incl. `mixins/framedblocks/data/framedblocks-content.hmod`; reproducible |

The slot's `stage/modded` tree is shared between ports that lease the slot, and
an earlier port's injected modules can survive there; this run cleared it first
and verified the staged `game.rs` named only `framedblocks` (2 modules) before
building, so the binary under test is this bundle.

### Ladder (pin `c9d474d`, `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=2`)

Run in one `flock -w 3600 /tmp/opencode/campaign-build.lock` chain
(`target/logs/*.exit`, `target/horizon-*.log`):

| Step | Exit | Evidence |
|---|---|---|
| `cargo fmt --all --check` | 0 | clean |
| `cargo test --workspace --locked --offline` | 0 | 12 passed |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 | 0 warnings |
| `horizon.py stage` | 0 | `stage_changed: true`, staging 71.4 s (final sources) |
| `horizon.py check` | 0 | `cargo_exit: 0`, cargo 16.3 s, 0 warnings |
| `horizon.py test` | 0 | `cargo_exit: 0`, cargo 24.3 s, 12 tests, 0 warnings |
| `horizon.py build` | 0 | `cargo_exit: 0`, cargo 210.4 s, `target/horizon-cargo/fast/horizon-client` |
| ctl build (`--bin horizon-ctl`, fast) | 0 | `target/horizon-cargo/fast/horizon-ctl` |

The first, cold ladder run (before the final readout wording) also passed every
step: stage 184.4 s, check 86.4 s, test 195.5 s, build 310.4 s, all
`cargo_exit: 0`, 0 warnings. The final ladder re-staged the readout change and
rebuilt `horizon-client` in 3 m 30 s; the binary used by the final e2e is that
build (`target/horizon-cargo/fast/horizon-client`, 81,646,096 bytes).

### In-game e2e (`tools/e2e.sh`, offscreen renderer)

`/tmp/opencode/framedblocks-e2e-final.log`, exit 0. Survival accounting on a
5×5 stone platform at y=199, the frame at (100,200,100), the player at
(100,200,102):

- before: **66,059** frame-brown pixels in the central crop, **0** gold;
  `/framedblocks camo 100 200 100` reads `none`.
- right click with a gold block: one gold consumed (4 → 3), the block entity
  reads `minecraft:gold_block (item minecraft:gold_block)`, the client draws
  **54,468** gold pixels.
- break: drops `framedblocks:framed_cube` ×1 and `minecraft:gold_block` ×1.
- framed hammer on a second camo: the block entity reads `none`, the gold
  block comes back (3 → 4), and the break drops only the frame.
- an uncamo'd frame: break drops only the frame.
- save + quit + reopen: the block entity still reads the gold camo and the
  reload screenshot has **67,495** gold pixels; the break returns frame + camo.

Counterfactual (`COUNTERFACTUAL=1`, `/tmp/opencode/framedblocks-e2e-counterfactual.log`,
exit 0): the identical scenario with `minecraft:diamond_block` reads and draws
diamond (**57,094** cyan pixels) and drops diamond; no gold. A renderer that
ignored the stored camo, or one hard-coded to gold, fails one of the two runs.

### Content (`.hmod`)

`content/framedblocks.hmod`: **7,612 bytes**, SHA-256
`6fd279ee4497390870021d3483c7671e790d00ee288837d0450c6a2b9221960d`,
built by `cargo horizon-mod build`, `cargo horizon-mod check`:
`framedblocks 11.4.0 (mod, API 0.2, both) … no code: content only, nothing to run`.

### Cleanup

Only this port's build state was removed (`target/horizon-cargo`,
`target/mixins`, `target/staged`, and the `/tmp/opencode/framedblocks-hmod-target`
cache); the campaign-wide `clean-targets.sh` was not run while other ports were
building. The upstream clone was deleted.

### Fail-capable legs re-run at the same pin (2026-11-10)

The embedded run is the deliverable proof; the two differential legs were re-run
at the same pin against the same one-file bundle:

| Run | Result |
|---|---|
| `EMBEDDED=1 tools/e2e.sh` (control + camo + save/reopen) | **PASS**, `EXIT:0` — `evidence/framedblocks/e2e-embedded.log` |
| `EMBEDDED=1 COUNTERFACTUAL=1 SKIP_RELOAD=1 tools/e2e.sh` | **COUNTERFACTUAL PASS** (diamond, not gold), `EXIT:0` — `e2e-counterfactual.log` |
| `EMBEDDED=1 SELFTEST=1 SKIP_RELOAD=1 CONTROL=0 tools/e2e.sh` | **FAIL** as intended: the stored camo is asserted to be a diamond block after a gold click, `EXIT:1` — `e2e-selftest.log` |

A second `tools/package.sh` run reproduced `framedblocks.hnmod` byte for byte
(SHA-256 `008508db…430e`).

## Review repairs (PR #579, 2026-11-11)

Review: `ports/_shared/horizon-mods/reviews/pr-579-framedblocks.md` (COMMENTED, 3 items).

| Reviewer's item | This job |
|---|---|
| LGPL source-review limitation | not a defect: the port is LGPL-3.0-only like the original, with the licence and the complete corresponding source in the repository and archive |
| artifact not staged | **orchestrator** (no write access) |
| relative link `(XFactHD/FramedBlocks)` | already absolute; re-checked |
| (prose) the description was a source-reading journal ("Repo …, commit …, `src/main/java/…:11`") | **rewritten** for players: the full-cube frame, its properties, the camo interaction, then the one-shape and default-state limitations |

Artifact: `framedblocks.hnmod` **25,179 B** `2c0f21fd059b75b866831cb6acf679fa13b298e149b5a547ba8c5e27f87a7451`
(content `framedblocks.hmod`).

## Migration to the SDK format (2026-11-18, SDK `0b14f8a`)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70) retired the `.hnmod` container, and the
registry closed this port's submission (**PR #820**) with *"republish this as a native `.hmod`
with the SDK from the next Nightly"*. The port is now one flat package.

- **Pin:** Horizon **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (nightly 2026-11-18),
  horizon-mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, mod API **`0.2`**,
  launcher 0.2.78. The SDK is fetched into `.horizon/sdk/` and never packed.
- **Version:** the port's own **0.2.0** (the retired bundle was 0.1.0; a resubmission is always
  a new version).
- **Class + tier** unchanged: class **content**, tier **deep** (`ports/ledger.csv`;
  `atm10-sources.csv`). This slice is still the one full-cube frame, its camo block entity, the
  client renderer, the hammer and the two recipes — no machine/deep feature is added.

### Files created, files deleted

| Created | What it holds |
|---|---|
| `horizon-mod.toml` | the only manifest: `id`, `version = "0.2.0"`, `api = "0.2"`, `side = "both"`, the required `capabilities`, `[native]` with the exact `game`/`horizon`/`mixins` |
| `Cargo.toml` | the crate: `horizon-mod-api` from `.horizon/sdk/` and nothing else |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks`, the `Frame`/`FrameItem` Parts and the glue, 4 glue tests |
| `src/runtime.rs` | the old `mixins/framedblocks/runtime.rs`, **byte-identical** (sha256 `26616be44389b2411e8f4da12652482d1f7676c064f8f1b10bb6d0bbb46d525f`, 240 lines; `cmp` clean against the retired copy and against the file inside the package) |
| `assets/framedblocks/`, `data/framedblocks/`, `data/minecraft/` | the old `content/` tree, moved to the package root and byte-identical (`cmp` per file ran in the same `mv`) |
| `LICENSE-NOTICE.md` | licence provenance + the carried-content statement; travels inside the package with `LICENSE` |
| `.gitignore` | `/target`, `/.horizon`, `/Cargo.lock` (trap 5) |
| `framedblocks.hmod` | the delivered package, port root |

| Deleted | Why | Where it went |
|---|---|---|
| `mixins/framedblocks/` (`horizon-mixin.toml`, `runtime.rs`, `support.rs`, one helper per hook, `data/framedblocks-content.hmod`) | the plan/injection schema and per-hook helpers are not part of any SDK package; the embedded content is unnecessary because `assets/`/`data/` ship in the flat package | `runtime.rs` → `src/`; the helper bodies → the Parts in `src/lib.rs`; the tree at `evidence/framedblocks/retired-hnmod-0.1.0/mixins/` |
| `framedblocks.hnmod` | retired container (launcher ≥ 0.2.70 cannot load it) | `evidence/.../framedblocks.hnmod` |
| `content/horizon-mod.toml`, `content/framedblocks.hmod`, `content/target/…hmod` | the second package; the flat package carries content itself | the manifest merged into the root `horizon-mod.toml`; the built files at `evidence/.../` |
| `compatibility.json`, `rust-toolchain.toml` | `[native]` in the manifest is the pin; the pinned Horizon decides the toolchain | `evidence/.../` |
| `tools/horizon.py`, `package.py`, `package.sh`, `notice.py`, `test_stage.py`, `test_template.py`, `tools/audit/`, `tools/__pycache__`, `.github/workflows/ci.yml` | the staging/packaging/ladder machinery; none of it applies to the flat package | `evidence/.../tools/`, `evidence/.../.github/` |
| `Cargo.lock` | not needed without registry deps and it changes the artifact (trap 5) | not kept |

**Kept:** `tools/e2e.sh` (the scenario is still the only in-game oracle; its header now says it
needs a launcher-built game and that the embedded-cache legs are retired), `notes.md`,
`FINDINGS.md`, `VALIDATION.md`, `README.md`, `LICENSE`.

**Build tooling note:** `ports/tools/build-slot.sh` speaks the retired staged tool (`stage`,
`plan`, the mixins CLI). For an SDK package every command runs **directly in the port** inside a
lease, which only supplies the slot and `CARGO_INCREMENTAL=0`:
`BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-fb --port ports/framedblocks --step test
-- bash -c 'cd ports/framedblocks && cargo test --offline'`. The `--port` is a label, not a `cd`
(RTV trap 1), and there is no stage, no `--target-dir` and no mixins pin in the build.

### Helper → hook table

| Retired injection (pin `1d63e0a5`) | SDK 0.2 | Where |
|---|---|---|
| `[[module]] runtime` | `src/runtime.rs` as an ordinary module (`pub mod runtime;`) | `src/lib.rs` |
| `[[module]] state` (`support.rs`) + `static STATE` | the Parts below; no shared mutable session state except the one-tick broken-frame stash | `src/lib.rs` |
| `block_entity` HEAD (`native_block_entity`, API 15/129) | `BlockEntityPart::block_entity`, `Some(Compound::new())`; the type is declared with `Registrar::block_entity_type("framed_cube").blocks(&["framed_cube"])` and attached with `block_entity_part` | `Frame` |
| `block_entity_update_tag` (API 45) | `BlockEntityPart::block_entity_update_tag` returning both keys, always (empty for a removal) | `Frame` |
| `block_entity_submit` (API 71) | `BlockEntityPart::block_entity_submit`; `native_draw_block` → `draw.block(BlockKey, &[])`, `native_draw_cube` → `draw.cube(…, [CubeFace::new(FRAME_SPRITE); 6], 0xFFFFFF)` | `Frame` |
| `use_item_on` HEAD (API 13), `claimed = true` | `BlockPart::use_item_on` returning `Some(InteractionResult::SuccessServer)`; `native_with_mod_block_entity` writes → `BlockEntityRef::merge_data`; `consume_held` → `LivingRef::set_hand_item`; `native_give_stack` → `PlayerRef::set_inventory_slot` | `Frame` |
| `place_block` HEAD (API 14) | `ItemPart::on_block_placed_by` on the `framedblocks:framed_cube` **item** (the upstream `setPlacedBy` moment); the placer's off hand is `LivingRef::hand_item(host, Hand::Off)` | `FrameItem` |
| `destroy_block` HEAD (API 14) | `BlockPart::on_destroyed_by_player`, reading the block entity while it is still in the world | `Frame` |
| `loot_drops` HEAD (API 75) | `Hooks::loot_drops` (full `ServerHost`, so an arbitrary camo item resolves to an `ItemKey`) | `Mod` |
| `server_tick` HEAD (API 1): inventory resend | **gone**: `set_hand_item` is a cell write with the engine's own slot sync, so no resend is needed | — |
| `server_tick` HEAD: stale break cleanup | `Hooks::server_tick`, `ServerTickEnd`, clears the one-tick stash | `Mod` |
| `register_commands` HEAD (API 8) | `Hooks::register_commands`, `Command::new("framedblocks camo <x:integer> <y:integer> <z:integer>")` | `Mod` |
| `perform_command` (API 8) | `Hooks::run_mod_command`: typed `CommandInvocation` (`literals`, `argument("x")`), `Commands::feedback` for the reply, `Flow::Cancel` when answered | `Mod` |
| `embedded_content.rs` + `data/framedblocks-content.hmod` | **gone**: `assets/` and `data/` are inside the flat `.hmod` | — |
| item tag / lang / recipes / loot | unchanged; carried at the package root | `assets/`, `data/` |

`runtime.rs` is untouched: `camo_action`, `render`, `camo_drop`, `consumes`,
`parse_camo_command`, `is_camo_block`, the constants and the 12 tests are called exactly as
before. `parse_camo_command` is still unit-tested, but the live command path now uses the SDK's
typed arguments (`CommandInvocation::argument`), which is why the glue has its own
`is_camo_command`/`camo_position` with tests.

### Behaviour changes caused by the mapping

1. **`SuccessServer` instead of `SUCCESS`.** A block Part runs on the server only, so it cannot
   swing a client prediction; `SuccessServer` swings the server's hand, the same visible result
   the retired `swing_arm()` had.
2. **The camo returns only into a free main-inventory slot.** Upstream's `Inventory.add` also
   merges into a partial stack; the SDK has no public "add to inventory" that reports whether
   everything went in (its `player_give_stack` throws the overflow at the player, which is
   *not* the "keep the camo" rule). The port therefore looks for an empty slot 0–35 and keeps
   the camo otherwise; with a partial matching stack and no empty slot it keeps the camo where
   upstream would merge (recorded as a deviation).
3. **No inventory resend hook.** `set_hand_item` writes the slot and the engine's tick sync
   sends it; the retired port's tick-end resend existed because its own `held_mut` write was
   invisible to the hub's slot diff.
4. **The camo write is transactional.** `merge_data` is `/data merge block`: it loads, marks
   the chunk changed and sends the update, so the explicit `native_block_entity_sync` call and
   the "mark the chunk modified" workaround are both gone.
5. **The readout checks the block entity type** (`BlockEntityRef::kind(host).id(host)`) instead
   of matching a `BlockEntity::Mod(id)` variant; a vanilla block entity at the position now
   answers "another block entity" rather than being mistaken for the frame.

### Build and check (verbatim)

```sh
# runtime + glue tests, and the lint gate, in a lease that only supplies the slot
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-fb --port ports/framedblocks --step test \
    -- bash -c 'cd ports/framedblocks && cargo test --offline'
# → test result: ok. 16 passed; 0 failed  (12 runtime + 4 glue)

BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-fb --port ports/framedblocks --step lint \
    -- bash -c 'cd ports/framedblocks && cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings'
# → Finished `dev` profile … (0 warnings, exit 0)

rm -f Cargo.lock
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-fb --port ports/framedblocks --step build \
    -- bash -c 'cd ports/framedblocks && cargo horizon-mod build'
# → SDK 0b14f8a (build 0b14f8acfd97fa2c750e42795249dcc981178ca4) in ./.horizon/sdk
# → Built ./target/horizon-mod/framedblocks.hmod

BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-fb --port ports/framedblocks --step check \
    -- bash -c 'cd ports/framedblocks && cargo horizon-mod check framedblocks.hmod &&
        cargo horizon-mod fits framedblocks.hmod --build nightly'
# → framedblocks 0.2.0 (native-mod, format native-1/0b14f8ac…/e59da909…, for 26.3)
# → 31432 bytes, SHA-256 82fb24e0…7a7e, unsigned        CHECK_EXIT:0
# → FramedBlocks: / 0b14f8a  made for                    FITS_EXIT:0
```

`BUILD_LOCAL=1` on every step: the automatic forward would sync the port to the far box, and
the SDK package needs its `.horizon/sdk` beside it; the local lease is the supported path here.

### Package

`framedblocks.hmod` — **31,432 B**, SHA-256
`82fb24e0051b77cf7d3b2eb76e3694077602c09cb0800b150aad3802c0857a7e`, **23 entries**:
`horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, the 13 carried
`assets/`+`data/` files, `src/lib.rs`, `src/runtime.rs`, `uses.json`. Built twice with identical
bytes (the same hash both times). The licence files travel **inside** the package (the LGPL
condition). `src/runtime.rs` inside the package hashes to `26616be4…d525f`, the retired file.

The package's own `uses.json` (324 entries) names the hooks and methods the Parts implement:
`hook:block_entity`, `hook:block_entity_update_tag`, `hook:block_entity_submit`,
`hook:use_item_on`, `hook:on_block_placed_by`, `hook:on_destroyed_by_player`, `hook:loot_drops`,
`hook:register_commands`, `hook:run_mod_command`, `hook:server_tick`; the matching
`method:BlockEntityPart::*`, `method:BlockPart::*`, `method:ItemPart::on_block_placed_by` and
`method:Hooks::*`; `handle:BlockEntityRef::merge_data`/`data`/`kind`,
`handle:ItemKey::id`/`BlockKey::id`, `handle:LivingRef::set_hand_item`/`hand_item`,
`handle:PlayerRef::is_creative`/`inventory_slots`/`set_inventory_slot`,
`handle:BlockEntityDrawContext::block`/`cube`, `handle:Registrar::block_entity_type`/
`block_entity_part`/`item_part`. It also lists `hook:blocks`, `hook:command` and
`method:Hooks::blocks`/`command`, which the mod does **not** implement — the checker
over-approximates from names in the source (`Command`, `.blocks(...)`); `cargo horizon-mod
check` passes with them, so they are not a subscription claim. Never read that file as the
hook list; the table above is.

Evidence: [`ports/_shared/evidence/framedblocks/sdk-0b14f8a/`](../_shared/evidence/framedblocks/sdk-0b14f8a/)
(`unit.log`, `lint.log`, `build.log`, `check.log`, `fits.log`, `sha256.txt`,
`package-entries.log`, `runtime-carry.log`, `sdk-uses.json`).

### Traps this migration cost

1. **The SDK must be beside the port.** `cargo test` needs `.horizon/sdk/`; a fresh clone gets it
   from the first `cargo horizon-mod` command. Here it was copied from the already-migrated
   `ports/imfast/.horizon/sdk` (same pin, gitignored, 21 MB) so the build stayed offline.
2. **`BlockEntityPart` is a supertrait pair.** The type needs **both** `#[mod_api::horizon_part]
   impl BlockPart for …` and `impl BlockEntityPart for …`; a `BlockEntityPart` alone does not
   attach. `Registrar::block_entity_part` also returns `Result<(), BlockEntityPartError>` — an
   unchecked call is an `unused_must_use` warning at `-D warnings`.
3. **`BlockEntityPart` needs a declared type.** Attaching a Part to an undeclared block entity
   type is refused at load (`attached … to undeclared native block entity type`); declare it first
   with `r.block_entity_type("<path>").blocks(&["<block path>"]).build()`.
4. **Handle methods, not host methods.** The inventory and level reads are on `PlayerRef`:
   `player.inventory_slots(host)`, `player.set_inventory_slot(host, slot, stack)`,
   `player.level(host)`, `placer.level(host)`. Calling `host.player_inventory_slots(...)` does not
   exist.
5. **`item_key` takes an `Identifier`, not a `&str`.** `host.item_key(&Identifier::parse(id).ok()?)`
   for a camo item id; `host.block_key(id)` is the `&str` one.
6. **`JavaString` has no `From<String>`.** NBT keys and string values take
   `str.into()`/`as_deref().unwrap_or("").into()`; a `String` must be borrowed first.
7. **A Part is not the mod.** Parts are `Send + Sync + 'static` structs built in `register`; the
   `Hooks` methods (the command, the loot hook, the tick) live on the `Mod` instance. State shared
   between them is a module `static` (the one-tick broken-frame stash), not a field.
8. **`CommandInvocation` is `#[non_exhaustive]`.** It cannot be built in a unit test; the glue's
   helpers take `&[String]`/`&[CommandArgument]` so they stay testable.
9. **`cargo fmt --all` would format the fetched SDK.** Use `cargo fmt` / `cargo fmt --check`
   (RTV trap 4). The SDK path dependency is a sibling of the crate, not a workspace member.
10. **The old layout is refused before the first build.** `mixins/`, `content/`, the `.hnmod` and
    `compatibility.json` had to move out of the project root first (`unexpected package path`).
11. **`cargo horizon-mod build` packs; it does not compile against the game.** The real native
    build is `horizon-launcher native-build . --instance <i> --trust-native-code`, which is not
    installed here and has no game build at `0b14f8a` (the shared binaries are at `1d63e0a5` and
    `41f822b6`). Compile evidence is `cargo clippy --all-targets` / `cargo test` against the
    pinned SDK API crate — the crate the launcher builds against — plus `check`/`fits`.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

### Verification (SDK `0b14f8a`)

| Check | Command | Result |
|---|---|---|
| Runtime tests first, before glue | `cargo test` with only `src/runtime.rs` in the crate | **12 passed**, 0 failed |
| Runtime + glue | `cargo test --offline` | **16 passed**, 0 failed (`evidence/…/unit.log`) |
| Lints | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`lint.log`) |
| Package | `cargo horizon-mod build` | `target/horizon-mod/framedblocks.hmod`, exit 0 (`build.log`) |
| Reproducible | second `cargo horizon-mod build` after `rm -f Cargo.lock` | same 31,432 B, same sha256 `82fb24e0…7a7e` |
| Registry check | `cargo horizon-mod check framedblocks.hmod` | `framedblocks 0.2.0 (native-mod, format native-1/0b14f8ac…/e59da909…, for 26.3)`, **31,432 B**, sha256 `82fb24e0…7a7e`, **exit 0** (`check.log`) |
| Fits the pinned build | `cargo horizon-mod fits framedblocks.hmod --build nightly` | `0b14f8a  made for`, exit 0 (`fits.log`) |
| `runtime.rs` untouched | `cmp src/runtime.rs <retired>` and `unzip -p framedblocks.hmod src/runtime.rs \| sha256sum` | `26616be4…d525f`, both clean (`runtime-carry.log`) |
| Skills line | `python3 ports/tools/check-skills.py --notes ports/framedblocks` | `Skills line present`, **SKILLS GATE: PASS** |

### Gaps after the migration

1. **No in-game run at `0b14f8a`.** No `horizon-launcher` and no game binary at this build; the
   `tools/e2e.sh` scenario was last proved on the retired bundle (`e2e-embedded.log`). The
   behaviour claims here are the *mapping* (each retired hook to a typed Part at the same vanilla
   call site), the type-checked glue, the package check, and the carried rules — not a gameplay
   run. The first slice at a launcher-capable box builds the game with the `.hmod` installed and
   records the transcript (the script's `EMBEDDED=1` path already launches with no `--dev-mod`).
2. **No log service in the SDK.** The retired hook's per-reset log line does not exist, so the
   e2e's assertions lean on the block entity readout (`/framedblocks camo`), the inventory counts
   and the screenshots (they already do).
3. **The spawner template is legacy** (see Content): it no longer decides block entity creation,
   but swapping it for a plain template is a content change that needs the in-game run to confirm.
4. ~~**State-specific camos are still not expressible from an item**~~ — **closed by 0.2.2**
   (2026-11-21): the block entity stores the full state string and the renderer draws it with its
   properties, so `/framedblocks camo <x> <y> <z> minecraft:grass_block[snowy=true]` works and is
   drawn (see [Slice 0.2.2](#slice-022--the-camo-is-a-block-state-2026-11-21)). What stays open is
   the *player* route to a non-default state: upstream's wrench rotation and camo applicator, and
   the blueprint, are still not ported.

### Changelog for the resubmission

> Supersedes #820, closed because Horizon retired `.hnmod` (#17992). Changes: the port is now one
> flat SDK package (`api = "0.2"`, `[native]` pinned to `0b14f8a` / `e59da90`, version 0.2.0)
> with its content (`assets/`, `data/`) inside it, so the embedded-content injection and the
> AEo-style cache write are gone. The hooks are typed Parts against hosts and events:
> `BlockEntityPart` on `framedblocks:framed_cube` for the block entity, its update tag and the
> client renderer, `BlockPart::use_item_on` for the camo interaction, `ItemPart::on_block_placed_by`
> for the off-hand camo on placement, and `Hooks::loot_drops`/`on_destroyed_by_player` for the camo
> return. The camo write is `BlockEntityRef::merge_data` (the `/data merge block` path with its own
> `setChanged` and client sync) instead of a pin-internal block-entity write, and the hand item is
> consumed with `LivingRef::set_hand_item` instead of the hub's inventory resend. `runtime.rs` and
> its 12 tests are byte-identical to the retired bundle's.

## Review fixes (2026-11-18) — registry source review #1209

Skills: superpowers/using-superpowers,systematic-debugging,test-driven-development,verification-before-completion; BeautifulRust/rust-best-practices

File: `var/review-all/1209-framedblocks.md`. Package **0.2.1**, `framedblocks.hmod` 32,045 B,
sha256 `c1fa1450fd3a25b8335a55bd0bec5c3c58544caa85140e7b4772fe89cdc2a225`.

1. **`src/lib.rs:251`/`:144`/`:172` an off-hand camouflage consumed a block item but stored only
   its block and item ids, so a component-bearing block item (a filled shulker box) came back as
   a plain item on removal or break.** The frame now refuses such a camo instead of storing it
   lossily: `runtime::may_be_camo(components, damage)` is false for a stack with a component patch
   or damage, and `camo_plan` returns `Plan::None` for it (the click is left to the game, nothing
   is consumed). Pinned by
   `a_camo_item_that_carries_data_is_refused_rather_than_come_back_plain` (unit). `cargo test`:
   **17 passed**. Clippy also caught one pre-existing `!x.is_none()` in the placement path, fixed.

**In-game leg.** `ports/tools/launcher-game.sh run framedblocks` → `PASS` + `E2E EXIT:0`
(`ports/_shared/evidence/framedblocks-e2e/e2e-launcher.log`); the package-less Nightly fails at the
first phase (`…-control.log`, `EXIT:1`). The camo tests still store and return a plain block item;
a component-bearing one is refused by the unit test above.

## Slice 0.2.2 — the camo is a block state (2026-11-21)

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

**Pin:** Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` + horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, mod API 0.2, launcher 0.2.92, game 26.3.
**Version 0.2.2.** Class **content**, tier **deep** (unchanged): the slice changes what the
block-entity renderer draws and what the block entity stores, and adds no machine or menu.

### The gap it comes from

The port's own gap list, verbatim:

> 4. **State-specific camos are still not expressible from an item** (inherited): a block item
> gives its block's default state; the port stores the block id and draws that default state. An
> SDK `BlockState` *could* be stored in the saved data (a block id plus properties), but the item
> path only ever supplies the default; a later slice should store a full state for a camo placed
> by a command/blueprint.

The jar's own vocabulary agrees that a camo is a state, not a block: `desc.framedblocks.stored_camo
= Camo: %s` and `tooltip.framedblocks.camo_rotation.true = The targetted camo can be rotated`
(FramedBlocks 10.6.3), and upstream's apply path reads
`BlockItem.getBlock().defaultBlockState()` into a camo container that holds a `BlockState`. Until
0.2.2 the port threw the properties away, so a stair camo could never face east and a log camo
could never lie down — and every later family that moves a state (the wrench's rotation, the camo
applicator, the blueprint) had nothing to move.

Tier: this is the deep half the ledger already assigns (custom renderer input + block-entity
data), not a new tier. The item path is unchanged; the state route is a command (and, later, a
blueprint).

### What changed

| File | Change |
|---|---|
| `src/camo_state.rs` (new) | the pure rule: `CamoState::parse` splits a `/setblock` state (`minecraft:oak_stairs[facing=east,half=top]`) into the block and its properties, and refuses anything a canonical state cannot be (no block, unclosed list, empty list, a property without a name or value, an empty property, text after the list). 8 tests. |
| `src/lib.rs` | the camo write stores the **state string**; `block_entity_submit` draws `event.draw.block(key, state.properties())`; a block item contributes `block_state_default` + `block_state_string` (exactly the old default-state camo); the readout canonicalises through `block_state_string` and says `(no item)` for a block with none; `/framedblocks camo <x> <y> <z> [state:text]` reads without a state and **sets** with one, refusing (`'…' is not a block state`) anything `block_state_id` will not parse, and `never a camo` for a framed block. |
| `src/runtime.rs` | the rules and their tests unchanged; one variant renamed `Held::Block` → `Held::BlockItem` (see the trap below). sha256 `d034d5da760d9480364dd10fa85b030e3f73c496a17e00dc4a0f51e1b9478abd`. |
| `horizon-mod.toml`, `Cargo.toml` | version 0.2.2; `[native]` moved to `0a875da` / `e59da909`; the description names state camos and the command. |
| `tools/e2e.sh` | phase 7 (the state: default → no snow, `snowy=true` → snow, refusal, drop), phase 8 (both carried recipes crafted through the table grid), `SELFTEST_STATE=1`, `SELFTEST_RECIPE=1`, `KEEP=1`, a `snow` colour counter; phase 9 is the old save/reopen. |

**Old worlds still load**: a stored bare block id (`minecraft:gold_block`) *is* a state string (the
block's default), so no migration is needed and the readout prints it unchanged.

### Red → green (TDD)

The tests came first, against a stub that kept the whole stored string as the block and no
properties — 0.2.1's behaviour. Verbatim failure (evidence `sdk-0a875da/red.log`):

```
---- camo_state::tests::the_properties_after_a_block_id_are_kept_in_order stdout ----
thread 'camo_state::tests::the_properties_after_a_block_id_are_kept_in_order' (3791854) panicked at src/camo_state.rs:64:9:
assertion `left == right` failed
  left: "minecraft:oak_stairs[facing=east,half=top]"
 right: "minecraft:oak_stairs"

test result: FAILED. 19 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out
```

`the_properties_after_a_block_id_are_kept_in_order`, `a_state_without_a_block_is_refused`,
`an_unclosed_property_list_is_refused`, `a_property_must_have_a_name_and_a_value`,
`an_empty_property_is_refused` and `text_after_the_property_list_is_refused` were the six. After the
parser: `test result: ok. 26 passed; 0 failed`.

### Ladder (0a875da)

`BUILD_LOCAL=1` on every step (the port's `.horizon/sdk` is the 0a875da SDK copied from an
already-migrated port; the automatic remote forward does not sync it).

| Step | Result |
|---|---|
| `cargo fmt --check` | clean (`sdk-0a875da/clippy.log`: `FMT_CLEAN`) |
| `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** |
| `cargo test --offline` | **26 passed**, 0 failed (17 carried + 8 `camo_state` + 1 new glue test) (`sdk-test.log`) |
| `cargo horizon-mod build` twice, `rm -f Cargo.lock` before each | **36,119 B**, sha256 `aea0763a00cc2239a3e014ef1554b8af85f3e9133c361008336b680666887983`, `cmp`-identical (`build.log`) |
| `cargo horizon-mod check framedblocks.hmod` | `framedblocks 0.2.2 (native-mod, format native-1/0a875da…/e59da909…, for 26.3)`, 36,119 B, `CHECK_EXIT:0` |
| `cargo horizon-mod fits framedblocks.hmod --build 0a875da` | `0a875da  made for`, exit 0 |
| `cargo horizon-mod fits framedblocks.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (the pin run is a tautology; this one is not) |
| `python3 ports/tools/verify-sdk-package.py framedblocks` | **1/1** |

### The trap this slice hit: a bare `Block` (and how the state argument had to be `text`)

Three in-game rounds and one `fits` run taught the slice's API edges; each is recorded because the
next port will meet them:

1. **`fits --build 0b14f8a` failed on 0.2.1's shape** — *"the type `Block` can no longer be built
   with a literal, and `src/lib.rs:310` builds it"*. The trigger is the *variant* `Held::Block
   { .. }`: `uses.json` reads a bare `Block` as the SDK's `type:Block` draft struct, which is
   `non_exhaustive` and not constructible (the same false positive that hit `sushigocrafting`,
   `toolbelt` and `mekanism`). Renaming the variant to `Held::BlockItem` cleared it; the 0.2.1
   evidence only ran `fits --build nightly`, which is the `made_for` build and cannot see it.
2. **A `set` literal after the arguments is not in the tree at this pin.** Two usages (`… <z>` and
   `… <z> set <state>`), which is how `ironjetpacks` registers a longer form, gave
   `Incorrect argument for command` at `set` in game.
3. **`word` cannot carry a state.** `minecraft:grass_block` in a `:word` argument answered
   `Expected whitespace to end one argument, but found trailing data` — brigadier's unquoted
   reader stops at `:`, `[`, `]` and `=`.
4. **What works**: one usage, `framedblocks camo <x:integer> <y:integer> <z:integer>
   [state:text]` — a greedy `text` argument as the optional last node (the shape `rftoolsbase`'s
   `[nonce:word]` proved and this slice extends to `text`). Measured in game.

### In game (launcher build at 0a875da)

`ports/tools/launcher-game.sh run framedblocks` → **PASS**, `EXIT:0`
(`ports/_shared/evidence/framedblocks-e2e/e2e-launcher-0.2.2.log`). Phases 1–6 and 8 are 0.2.1's
(gold camo by right click, the readout, the draw, the break drop, the hammer, an uncamo'd frame,
save/reopen); phase 7 is the new family, verbatim:

```
== 7. the camo is a block state and the client draws its properties ==
  read: framedblocks camo 100 200 100 = minecraft:grass_block[snowy=false] (item minecraft:grass_block)
  snow pixels in state-plain.png: 153
  read: framedblocks camo 100 200 100 = minecraft:grass_block[snowy=true] (item minecraft:grass_block)
  snow pixels in state-snowy.png: 20870
  snow pixels: 20870 with the state, 153 without
  refusal: framedblocks camo 100 200 100 = 'minecraft:stone[nope=1]' is not a block state
  read: framedblocks camo 100 200 100 = minecraft:grass_block[snowy=true] (item minecraft:grass_block)
  break: … "drops":{"framedblocks:framed_cube":1,"minecraft:grass_block":1} …
```

Phase 8 crafts both carried recipes through the crafting table's own grid (the parity rule's
sharpest case — a carried recipe that is never crafted is dead data):

```
== 8. the carried recipes craft (frame, then hammer) ==
  frame grid result: framedblocks:framed_cube 4
  framed_cube: 1 -> 5; planks 0 -> 0; sticks 0 -> 0
  hammer grid result: framedblocks:framed_hammer 1
  framed_hammer: 1 -> 2; sticks left 0
```

The grid is filled one item at a time (`fill_grid` picks the stack up and right-clicks one into
each slot), the **result slot 200** is asserted to hold the right id and count *before* it is
taken, and the post-craft counts prove the ingredients were consumed (planks 4→0, sticks 4→0 and
0 for the hammer, frames 4→2).

Each assertion can fail, and each was watched failing:

| Leg | Result |
|---|---|
| main (`e2e-launcher-0.2.2.log`) | **PASS**, `EXIT:0` |
| control — the same Nightly **without** the package (`GAME=$CONTROL_GAME`) | `EXIT:1` at phase 1 (there is no `framedblocks:framed_cube` to place) (`e2e-launcher-control-0.2.2.log`) |
| `SELFTEST=1` (the stored camo asserted to be diamond after a gold click) | **FAIL**, `EXIT:1` (`e2e-selftest-0.2.2.log`) |
| `SELFTEST_STATE=1` (the stored state asserted to be `snowy=true` after `snowy=false` was set) | **FAIL**, `EXIT:1` (`e2e-selftest-state-0.2.2.log`) |
| `SELFTEST_RECIPE=1` (the frame grid asserted to give a framed hammer) | **FAIL**, `EXIT:1` (`e2e-selftest-recipe-0.2.2.log`) |
| `python3 ports/tools/check-e2e-logs.py framedblocks-e2e --strict` | `logs=7 ok=7 missing-exit=0 bad-exit=0`, exit 0 |

The snow is the differential: the two screenshots are the same scene and differ only in the stored
state's `snowy` property, so a renderer that ignored the properties (0.2.1: 153 vs 153) fails
`expect_more_colour`, and one that drew nothing fails `expect_colour`. A `snow` colour counter is
used instead of `white` because the survival HUD and the command's own chat line are white in
*both* shots (2,601/2,466 measured) — the background cancels out, the state does not.

### Coverage (ruling 2026-11-20)

| Family (jar) | State | Where it is proved |
|---|---|---|
| the framed cube block, its item, its block entity | `tested` | e2e phase 1 (`setblock`, texture), phase 2 (empty compound), phase 7/8 (state preserved) |
| applying a camo with a block item | `tested` | phase 3 (one item consumed, readout, 54,469 gold px) |
| removing a camo with the framed hammer | `tested` | phase 5 (item back, readout `none`) |
| the camo item returned on a player break | `tested` | phases 4, 6, 7 (state camo drops `minecraft:grass_block`), 8 |
| the camo as a **block state** (new) | `tested` | phase 7 (readout + 153 → 20,870 snow px + refusal + `SELFTEST_STATE`) |
| `/framedblocks camo … [state]` | `tested` | phases 2–8 (read), phase 7 (set, refuse) |
| the two recipes (`framed_cube`, `framed_hammer`) | `tested` | phase 8: the 3x3 grid crafts each with its declared pattern, the result slot holds `framed_cube 4` / `framed_hammer 1`, the ingredients are consumed (planks 4→0, sticks 4→0 and 0, frames 4→2), and `SELFTEST_RECIPE=1` fails when the frame grid is asserted to give a hammer |
| `/framedblocks` tab completion | `tested` | implicitly: the tree's kinds are what the refusal and the set prove |
| every other frame shape, the frame flags, waterlogging | `not ported` | out of this slice (gap 1) |
| the wrench, camo rotation, camo applicator, blueprint, item camo preview | `not ported` | gap 1/2; the state now exists, the tool does not |
| frame overlay, sounds/particles, camo rotation overlay | `not ported` | deviations 4/5 |
| a camo that is itself a block entity | `blocked(draw.block draws the baked model only)` | deviation 6 |
| the camo on a non-player break (explosion, mob, piston) | `blocked(block_drops carries a LevelHost, which cannot resolve an arbitrary ItemKey)` | deviation 3; the *explosion* pre-capture via `explosion_detonate` is the route a later slice should try |

### Deviations this slice adds

1. **The state route is a command.** Upstream's non-default states come from the wrench (rotation),
   the camo applicator (a clicked block's state) and the blueprint; none of the three exists yet,
   so a player can only *read* a state the port already stores, and set one with
   `/framedblocks camo <x> <y> <z> <state>`.
2. **A state whose block has no item** (`minecraft:wheat`) stores and draws, and its break drops
   nothing (`block_item` is None, so `CamoItem` is empty and the readout says `(no item)`); upstream
   would drop the block's item, which does not exist.
3. **The readout is canonical**: `block_state_string` writes every property the block has
   (`minecraft:grass_block[snowy=false]`), where 0.2.1 printed the bare id. The e2e's gold
   assertions still match because a property-less block serialises as its bare id.

### Changelog for the resubmission

> Version 0.2.2 (supersedes 0.2.1, which superseded #820). The camo is now a full **block state**:
> the block entity stores `/setblock`'s serialised state (`minecraft:grass_block[snowy=true]`) and
> the client draws it with those properties, so a stair's facing and a log's axis are expressible.
> A block item still applies its block's default state; `/framedblocks camo <x> <y> <z> [state]`
> reads the stored state and sets one (a property the block does not have is refused and the old
> camo stays), and a break drops the state's item. `src/camo_state.rs` is the new pure parser (8
> tests); `src/runtime.rs`'s rules are unchanged apart from renaming `Held::Block` to
> `Held::BlockItem` (`fits --build` read the bare name as the SDK's non-constructible `type:Block`).
> The `[native]` pins move to `0a875da` / `e59da909`.
