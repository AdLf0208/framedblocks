// Game-side glue for FramedBlocks' camo. Injected into `crate::game` as
// `framedblocks_state`; it needs the game, so it is not part of the standalone
// library (only `runtime.rs` is).
//
// Upstream keeps the camo in the framed block's block entity
// (`FramedBlockEntity.camoContainer`, a `CamoContainer` holding a
// `BlockState`), not in the block state: the state only carries the frame's
// own flags. The pin has the same shape: `native_block_entity` (API 129) gives
// a content block a block entity on any template, `native_block_entity_update_tag`
// (API 45) sends its saved compound to clients, and
// `native_block_entity_submit` (API 71) draws it with `native_draw_block`.
// The frame's blockstate model is empty, so the client's only geometry is what
// the block entity renderer draws: the camo block's default-state model, or
// the frame's texture when no camo is set.
//
// The camo compound is `Camo` (the camo block's id) plus `CamoItem` (the item
// a break gives back, the item that applied it). A removal writes empty
// strings rather than dropping the keys: a client merges an update tag field
// by field (`ModBlockEntity::merge_update_tag`), so an absent key would leave
// the old camo on screen.
//
// A break's camo drop rides the loot roll: `destroy_block` (API 14) reads the
// block entity while the block is still there, and `loot_drops` (API 75) adds
// the camo item to the stacks about to drop in the same tick. A break that
// rolls no loot (creative) leaves its entry behind; the server tick's end
// forgets it.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use horizon_client::containers::{BlockEntity, ModBlockEntity};
use horizon_client::items::ItemStack;
use horizon_client::text::Text;
use horizon_world::nbt::{Compound, JavaString, Tag};
use horizon_world::registry::BlockStateId;

use crate::game::framedblocks::{
    AIR, CAMO_ITEM_TAG, CAMO_TAG, CamoAction, FRAMED_CUBE, FRAMED_HAMMER, FRAME_SPRITE, Held,
    Render, camo_action, camo_drop, consumes, parse_camo_command, render,
};
use crate::game::native_block_entity_draw::{BlockEntityDraw, CubeFace};
use crate::game::native_hooks::NativeLootDrops;
use crate::game::native_lifecycle::ServerTickPhase;
use crate::game::native_server::{NativeCommand, NativeServerPlayer};
use crate::game::Game;

/// Upstream's `FramedBlockEntity` is one block entity type for every frame;
/// this slice has one frame, so the entity id is the block's id.
#[derive(Default)]
struct State {
    /// A frame a player broke this tick: its camo, for the loot roll.
    broken: BTreeMap<[i32; 3], (String, String)>,
    /// Players whose inventory a camo use changed, to be resent at the tick's
    /// end (the hub's slot diff is refreshed during the guest turn).
    sync: BTreeSet<u128>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn shared() -> std::sync::MutexGuard<'static, Option<State>> {
    STATE.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// `/framedblocks camo <x> <y> <z>`: the camo block and item stored on the
/// frame at that position, or `none`. The readout is the server's block
/// entity, so it proves the camo is stored, not only drawn.
pub(super) fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new(
        "framedblocks camo <x:integer> <y:integer> <z:integer>",
    ));
}

pub(super) fn on_perform_command(
    game: &mut Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let Some(pos) = parse_camo_command(command) else {
        return;
    };
    *ran = true;
    let key = (pos[0], pos[1], pos[2]);
    let line = match game.world.block_entities.get(&key) {
        Some(BlockEntity::Mod(entity)) if entity.id == FRAMED_CUBE => {
            let (camo, camo_item) = read_camo(entity);
            match (camo, camo_item) {
                (Some(camo), Some(item)) => format!(
                    "framedblocks camo {} {} {} = {camo} (item {item})",
                    pos[0], pos[1], pos[2]
                ),
                _ => format!("framedblocks camo {} {} {} = none", pos[0], pos[1], pos[2]),
            }
        }
        Some(BlockEntity::Mod(entity)) => format!(
            "framedblocks camo {} {} {} = mod {} (not the frame)",
            pos[0], pos[1], pos[2], entity.id
        ),
        Some(_) => format!(
            "framedblocks camo {} {} {} = another block entity",
            pos[0], pos[1], pos[2]
        ),
        None => format!(
            "framedblocks camo {} {} {} = no block entity",
            pos[0], pos[1], pos[2]
        ),
    };
    game.native_feedback(Text::literal(line));
}

/// `EntityBlock.newBlockEntity`: the frame gets a block entity on its plain
/// cube template.
pub(super) fn on_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if block == FRAMED_CUBE {
        *entity = Some((FRAMED_CUBE.to_owned(), Compound::new()));
    }
}

/// `BlockEntity.getUpdateTag`: the camo a client needs to draw it. Always a
/// tag for the frame, so a removal's empty values reach the client.
pub(super) fn on_block_entity_update_tag(entity: &ModBlockEntity, tag: &mut Option<Compound>) {
    if entity.id != FRAMED_CUBE {
        return;
    }
    let (camo, camo_item) = read_camo(entity);
    let mut compound = Compound::new();
    compound.insert(
        JavaString::from(CAMO_TAG),
        Tag::String(JavaString::from(camo.as_deref().unwrap_or(""))),
    );
    compound.insert(
        JavaString::from(CAMO_ITEM_TAG),
        Tag::String(JavaString::from(camo_item.as_deref().unwrap_or(""))),
    );
    *tag = Some(compound);
}

/// `BlockEntityRenderer.submit`: the camo block's model, or the frame's own
/// texture when no camo is set (or the camo's block is not in this client's
/// resources).
pub(super) fn on_block_entity_submit(
    game: &Game<'_>,
    block: &str,
    entity: &ModBlockEntity,
    draw: &mut BlockEntityDraw,
) {
    if block != FRAMED_CUBE || entity.id != FRAMED_CUBE {
        return;
    }
    let (camo, _) = read_camo(entity);
    match render(camo.as_deref()) {
        Render::Camo(camo_block) => {
            if !game.native_draw_block(draw, camo_block, &[]) {
                draw_frame(game, draw);
            }
        }
        Render::Frame => draw_frame(game, draw),
    }
}

/// The frame's own cube: the pin's `native_draw_cube` with the frame sprite on
/// every face (upstream's `framed_cube` model).
fn draw_frame(game: &Game<'_>, draw: &mut BlockEntityDraw) {
    let face = CubeFace {
        sprite: FRAME_SPRITE,
        uv: None,
    };
    game.native_draw_cube(
        draw,
        [0.0, 0.0, 0.0],
        [16.0, 16.0, 16.0],
        [Some(face); 6],
        0x00FF_FFFF,
    );
}

/// `FramedBlockEntity.handleInteraction`: apply the held block as the camo,
/// remove it with the framed hammer, or leave the click alone.
pub(super) fn on_use_item_on(
    game: &mut Game<'_>,
    stack: &ItemStack,
    pos: [i32; 3],
    claimed: &mut bool,
) {
    if !is_frame(game, block_at(game, pos)) {
        return;
    }
    let (camo, camo_item) = camo_at(game, pos);
    match camo_action(held(game, stack), camo.as_deref()) {
        CamoAction::Apply { block, item } => {
            let (block, item) = (block.to_owned(), item.to_owned());
            write_camo(game, pos, &block, &item);
            if consumes(creative(game)) {
                consume_held(game);
                sync_later(game);
            }
            game.native_block_entity_sync(pos);
            *claimed = true;
        }
        CamoAction::Remove => {
            let Some(item) = camo_drop(camo_item.as_deref()) else {
                return;
            };
            let stack = ItemStack {
                item: item.into(),
                count: 1,
                damage: 0,
                extra: None,
            };
            // Upstream keeps the camo when the player has no room for it
            // (`AbstractBlockCamoContainerFactory.removeCamo`).
            let uuid = game.native_acting_id();
            if game.native_give_stack(uuid, stack).is_none() {
                clear_camo(game, pos);
                game.native_block_entity_sync(pos);
            }
            *claimed = true;
        }
        CamoAction::None => {}
    }
}

/// `AbstractFramedBlock.setPlacedBy` -> `tryApplyCamoImmediately`: a frame
/// placed with a block item in the off hand takes it as its camo.
pub(super) fn on_place_block(game: &mut Game<'_>, pos: [i32; 3], state: BlockStateId) {
    if !is_frame(game, state) {
        return;
    }
    let Some(offhand) = game.offhand.clone() else {
        return;
    };
    let (camo, _) = camo_at(game, pos);
    if camo.is_some() {
        return;
    }
    let CamoAction::Apply { block, item } = camo_action(held(game, &offhand), None) else {
        return;
    };
    let (block, item) = (block.to_owned(), item.to_owned());
    write_camo(game, pos, &block, &item);
    if consumes(creative(game)) {
        let empty = match game.offhand.as_mut() {
            Some(stack) => {
                stack.count = stack.count.saturating_sub(1);
                stack.count == 0
            }
            None => false,
        };
        if empty {
            game.offhand = None;
        }
        sync_later(game);
    }
    game.native_block_entity_sync(pos);
}

/// `ServerPlayerGameMode.destroyBlock`'s start: remember the camo before the
/// block entity goes, for this tick's loot roll.
pub(super) fn on_destroy(game: &mut Game<'_>, pos: [i32; 3], state: BlockStateId) {
    if !is_frame(game, state) {
        return;
    }
    let (camo, camo_item) = camo_at(game, pos);
    let (Some(camo), Some(camo_item)) = (camo, camo_item) else {
        return;
    };
    let mut guard = shared();
    let state = guard.get_or_insert_with(State::default);
    state.broken.insert(pos, (camo, camo_item));
}

/// `LootTableEvents.MODIFY_DROPS`: the camo item drops beside the frame
/// (`FramedBlockEntity.addCamoDrops`).
pub(super) fn on_loot_drops(
    game: &mut Game<'_>,
    context: &NativeLootDrops<'_>,
    drops: &mut Vec<ItemStack>,
) {
    let Some((pos, state)) = context.block else {
        return;
    };
    if !is_frame(game, state) {
        return;
    }
    let item = {
        let mut guard = shared();
        let state = guard.get_or_insert_with(State::default);
        state.broken.remove(&pos).map(|(_, item)| item)
    };
    let Some(item) = item else {
        return;
    };
    drops.push(ItemStack {
        item: item.into(),
        count: 1,
        damage: 0,
        extra: None,
    });
}

/// The server tick's end: resend the inventories a camo use changed and
/// forget a break whose loot never rolled (a creative break).
pub(super) fn on_server_tick(game: &mut Game<'_>, phase: ServerTickPhase) {
    if !matches!(phase, ServerTickPhase::ServerTickEnd { .. }) {
        return;
    }
    let sync = {
        let mut guard = shared();
        let state = guard.get_or_insert_with(State::default);
        state.broken.clear();
        std::mem::take(&mut state.sync)
    };
    for uuid in sync {
        let Some(host) = game.lan.as_mut() else {
            break;
        };
        if let Some(index) = host
            .hub
            .guests
            .iter()
            .position(|guest| guest.placed && !guest.dead && guest.uuid == uuid)
        {
            host.hub.send_inventory(index);
        }
    }
}

/// The block at `pos`, as the level in place has it.
fn block_at(game: &Game<'_>, pos: [i32; 3]) -> BlockStateId {
    game.world.block(pos[0], pos[1], pos[2])
}

/// Whether `state` is this port's frame.
fn is_frame(game: &Game<'_>, state: BlockStateId) -> bool {
    game.res.facts().get(state).name.as_ref() == FRAMED_CUBE
}

/// The frame's camo and camo item, from its block entity.
fn camo_at(game: &Game<'_>, pos: [i32; 3]) -> (Option<String>, Option<String>) {
    match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
        Some(BlockEntity::Mod(entity)) if entity.id == FRAMED_CUBE => read_camo(entity),
        _ => (None, None),
    }
}

/// The camo's block and item from a block entity's saved compound.
fn read_camo(entity: &ModBlockEntity) -> (Option<String>, Option<String>) {
    (
        string_tag(&entity.nbt, CAMO_TAG),
        string_tag(&entity.nbt, CAMO_ITEM_TAG),
    )
}

fn string_tag(compound: &Compound, key: &str) -> Option<String> {
    match compound.get(&JavaString::from(key)) {
        Some(Tag::String(value)) => {
            let value = value.to_string_lossy();
            (!value.is_empty()).then_some(value)
        }
        _ => None,
    }
}

/// Stores the camo on the frame's block entity.
fn write_camo(game: &mut Game<'_>, pos: [i32; 3], block: &str, item: &str) {
    let _ = game.native_with_mod_block_entity(pos, |entity| {
        entity.nbt.insert(
            JavaString::from(CAMO_TAG),
            Tag::String(JavaString::from(block)),
        );
        entity.nbt.insert(
            JavaString::from(CAMO_ITEM_TAG),
            Tag::String(JavaString::from(item)),
        );
    });
}

/// Clears the camo with empty values a client merges over the old ones.
fn clear_camo(game: &mut Game<'_>, pos: [i32; 3]) {
    let _ = game.native_with_mod_block_entity(pos, |entity| {
        entity
            .nbt
            .insert(JavaString::from(CAMO_TAG), Tag::String(JavaString::from("")));
        entity.nbt.insert(
            JavaString::from(CAMO_ITEM_TAG),
            Tag::String(JavaString::from("")),
        );
    });
}

/// What the acting player holds, as the camo rules name it.
fn held<'a>(game: &'a Game<'_>, stack: &'a ItemStack) -> Held<'a> {
    if stack.item.as_ref() == FRAMED_HAMMER {
        return Held::Hammer;
    }
    let Some(state) = game.res.items().get(&stack.item).and_then(|item| item.block) else {
        return Held::Other;
    };
    let Some(block) = game.pack.assets.registry.state(state) else {
        return Held::Other;
    };
    if block.name == AIR {
        return Held::Other;
    }
    Held::Block {
        block: block.name.as_str(),
        item: stack.item.as_ref(),
    }
}

/// Whether the acting player is in creative (`native_server_players`).
fn creative(game: &mut Game<'_>) -> bool {
    let uuid = game.native_acting_id();
    game.native_server_players()
        .iter()
        .any(|player| player.uuid == uuid && player.game_mode == 1)
}

/// Takes one from the acting hand (the player's state is swapped in inside a
/// use hook, so `held_mut` reaches them).
fn consume_held(game: &mut Game<'_>) {
    let empty = match game.held_mut() {
        Some(stack) => {
            stack.count = stack.count.saturating_sub(1);
            stack.count == 0
        }
        None => false,
    };
    if empty {
        *game.held_mut() = None;
    }
}

/// Queues a resend of the acting player's inventory for the tick's end.
fn sync_later(game: &mut Game<'_>) {
    let uuid = game.native_acting_id();
    let mut guard = shared();
    let state = guard.get_or_insert_with(State::default);
    state.sync.insert(uuid);
}
