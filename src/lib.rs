//! FramedBlocks for Horizon — a framed cube whose camo is a real block.
//!
//! Upstream's `FramedBlockEntity` keeps a camo in the framed block's block entity, and the
//! client draws the camo block's model in the frame's place. The camo is a block **state**
//! (`minecraft:grass_block[snowy=true]`), split by [`camo_state`] and drawn with the properties
//! the state carries; a block item still gives its block's default state. This is the same shape at the
//! SDK: a [`BlockEntityPart`] on `framedblocks:framed_cube` owns the saved data, sends it to
//! clients, and draws it, and a [`BlockPart`] answers the frame's right click. The camo item a
//! break gives back rides the loot roll; a frame placed with a block item in the off hand takes
//! it as its camo through the frame item's [`ItemPart`].
//!
//! The rules themselves live in [`runtime`], byte-identical to the retired bundle's
//! `runtime.rs` and tested by `cargo test` without the game.
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::{
    self, BlockEntityDrawContext, BlockEntityEvent, BlockEntityPart, BlockEntitySubmitEvent,
    BlockEntityUpdateTagEvent, BlockKey, BlockPart, BlockState, ClientHost, Command,
    CommandArgument, CommandArgumentValue, Compound, CubeFace, Flow, Hand, Hooks, Identifier,
    InteractionResult, ItemPart, ItemStack, LevelRef, LevelSide, LivingRef, LootDropsEvent,
    OnBlockPlacedByEvent, OnDestroyedByPlayerEvent, PlayerRef, RegisterCommandsEvent, Registrar,
    RegistrationContext, RunModCommandEvent, ServerHost, ServerTickEvent, ServerTickPhase, Tag,
    Text, UseItemOnEvent,
};

pub mod camo_state;
pub mod runtime;

use camo_state::CamoState;
use runtime::{
    CAMO_ITEM_TAG, CAMO_TAG, CamoAction, FRAME_SPRITE, FRAMED_CUBE, FRAMED_HAMMER, Held, Render,
};

/// The last slot of the player's main inventory (`Inventory`: 0 to 35, the hotbar 0 to 8).
const MAIN_INVENTORY_END: u32 = 35;

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// The frame's block entity type, its two Parts, and the frame item that applies an
    /// off-hand camo on placement.
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        let kind = r
            .block_entity_type("framed_cube")
            .blocks(&["framed_cube"])
            .build();
        // The type was declared on the line above; a refusal here is a programming error.
        r.block_entity_part(&kind, Frame)
            .expect("the frame's block entity type is declared just above");
        let frame_item = r.item_key("framedblocks:framed_cube");
        r.item_part(frame_item, FrameItem);
    }

    /// `/framedblocks camo <x> <y> <z> [state]`: without a state, the readout the e2e and a
    /// player use to see what the frame's block entity really stores; with one, the way a command
    /// or a blueprint puts a **state-specific** camo on a frame (a stair's facing, a log's axis)
    /// that a block item cannot supply.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        // One usage with an optional trailing state: the readout, and the write that a state
        // after the position turns it into (an optional argument is what the tree takes last).
        event.commands.push(Command::new(
            "framedblocks camo <x:integer> <y:integer> <z:integer> [state:text]",
        ));
    }

    /// Answers the readout from the server's block entity, so it proves the camo is stored and
    /// not only drawn.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        if event.cancelled || !is_camo_command(&event.command.literals) {
            return Flow::Continue;
        }
        let Some(pos) = camo_position(&event.command.arguments) else {
            return Flow::Continue;
        };
        let Some(dimension) = event.command.dimension.id(host) else {
            return Flow::Continue;
        };
        let Some(level) = host.level(&dimension) else {
            return Flow::Continue;
        };
        let line = match camo_state_argument(&event.command.arguments) {
            Some(state) => set_camo(host, level, pos, &state),
            None => describe_camo(host, level, pos),
        };
        // Routed to the command's source, as vanilla's own feedback is.
        let _ = host.commands().feedback(Text::literal(line));
        Flow::Cancel
    }

    /// The camo item a player's break of the frame gives back, beside the frame's own loot.
    fn loot_drops(&self, host: &mut dyn ServerHost, event: &mut LootDropsEvent<'_>) {
        let Some((pos, state)) = event.context.block else {
            return;
        };
        if !is_frame(host, state) {
            return;
        }
        if let Some(stack) = take_broken(pos) {
            event.drops.push(stack);
        }
    }

    /// A break that rolled no loot (creative) leaves its camo stashed; forget it at the tick's
    /// end so it can never attach to a later break at the same position.
    fn server_tick(&self, _host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        let ServerTickPhase::ServerTickEnd { .. } = event.phase else {
            return;
        };
        broken_frames()
            .lock()
            .expect("the broken-frame stash lock is held for one map clear")
            .clear();
    }
}

/// The frame: its saved camo, the tag clients draw from, the renderer, and the right click that
/// applies or removes a camo.
struct Frame;

#[mod_api::horizon_part]
impl BlockPart for Frame {
    /// `FramedBlockEntity.handleInteraction`: a block item on an empty frame applies its block;
    /// the framed hammer removes a set camo; anything else leaves the click to the game.
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        let click = event.click;
        let level = host.level(click.dimension.key().id())?;
        let (camo, camo_item) = camo_at(host, level, click.pos);
        match camo_plan(host, click.stack, camo.as_deref()) {
            Plan::Apply { state, item } => {
                if !write_camo(host, level, click.pos, &state, &item) {
                    return None;
                }
                let player = host.player(click.player)?;
                if runtime::consumes(player.is_creative(&*host) == Some(true)) {
                    let living = host.living(player.entity())?;
                    consume_one(host, living, click.hand, click.stack);
                }
                Some(InteractionResult::SuccessServer)
            }
            Plan::Remove => {
                let item = runtime::camo_drop(camo_item.as_deref())?;
                let key = host.item_key(&Identifier::parse(item).ok()?)?;
                let player = host.player(click.player)?;
                // Upstream keeps the camo when the player has no room for the item.
                if !give_back(host, player, ItemStack::of(key)) {
                    return None;
                }
                if !clear_camo(host, level, click.pos) {
                    return None;
                }
                Some(InteractionResult::SuccessServer)
            }
            Plan::None => None,
        }
    }

    /// A player break: remember the camo while the block entity is still in the world, for this
    /// tick's loot roll.
    fn on_destroyed_by_player(
        &self,
        host: &mut dyn ServerHost,
        event: &mut OnDestroyedByPlayerEvent<'_>,
    ) -> Option<bool> {
        let player = host.player(event.player)?;
        let level = player.level(&*host)?;
        let entity = host.block_entity(level, event.pos)?;
        let data = entity.data(host)?;
        let (camo, camo_item) = read_camo(&data);
        // The camo block itself is only kept for the readout; the item is what drops.
        let _ = camo;
        let item = runtime::camo_drop(camo_item.as_deref())?;
        let key = host.item_key(&Identifier::parse(item).ok()?)?;
        stash_broken(event.pos, ItemStack::of(key));
        // The break itself is unchanged.
        None
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for Frame {
    /// `EntityBlock.newBlockEntity`: the frame's saved data starts empty.
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    /// `BlockEntity.getUpdateTag`: always both keys, so a removal's empty values reach a client
    /// whose copy still shows the old camo (an update tag merges field by field).
    fn block_entity_update_tag(
        &self,
        event: &mut BlockEntityUpdateTagEvent<'_>,
    ) -> Option<Compound> {
        let (camo, camo_item) = read_camo(&event.entity.nbt);
        let mut tag = Compound::new();
        tag.insert(
            CAMO_TAG.into(),
            Tag::String(camo.as_deref().unwrap_or("").into()),
        );
        tag.insert(
            CAMO_ITEM_TAG.into(),
            Tag::String(camo_item.as_deref().unwrap_or("").into()),
        );
        Some(tag)
    }

    /// `BlockEntityRenderer.submit`: the camo state's model, or the frame's own texture when no
    /// camo is set (or the camo's block or one of its properties is not in this client's
    /// resources).
    fn block_entity_submit(&self, host: &dyn ClientHost, event: &mut BlockEntitySubmitEvent<'_>) {
        let (camo, _) = read_camo(&event.entity.nbt);
        let state = camo.as_deref().and_then(CamoState::parse);
        let block = state.as_ref().map(CamoState::block);
        match runtime::render(block) {
            Render::Camo(block) => {
                let drawn = match &state {
                    Some(state) => host
                        .block_key(block)
                        .is_some_and(|key| event.draw.block(key, state.properties())),
                    None => false,
                };
                if !drawn {
                    draw_frame(event.draw);
                }
            }
            Render::Frame => draw_frame(event.draw),
        }
    }
}

/// The frame item: a frame placed with a block item in the off hand takes it as its camo
/// (upstream's `AbstractFramedBlock.setPlacedBy` -> `tryApplyCamoImmediately`).
struct FrameItem;

#[mod_api::horizon_part]
impl ItemPart for FrameItem {
    fn on_block_placed_by(&self, host: &mut dyn ServerHost, event: &mut OnBlockPlacedByEvent<'_>) {
        let Some(placer) = event.placer else {
            return;
        };
        let Some(level) = placer.level(&*host) else {
            return;
        };
        if camo_at(host, level, event.pos).0.is_some() {
            return;
        }
        let Some(living) = host.living(placer.entity()) else {
            return;
        };
        let Some(offhand) = living.hand_item(&*host, Hand::Off) else {
            return;
        };
        let Plan::Apply { state, item } = camo_plan(host, Some(&offhand), None) else {
            return;
        };
        if !write_camo(host, level, event.pos, &state, &item) {
            return;
        }
        if runtime::consumes(placer.is_creative(&*host) == Some(true)) {
            consume_one(host, living, Hand::Off, Some(&offhand));
        }
    }
}

/// What the glue does with a click, in owned strings: [`runtime::camo_action`] borrows its
/// inputs, and a camo state is a fresh string from the registry.
enum Plan {
    /// Store the camo `state` (the held block's default state) and remember `item` for the drop.
    Apply {
        state: String,
        item: String,
    },
    Remove,
    None,
}

/// `FramedBlockEntity.handleInteraction`'s decision, with the registry lookups the runtime does
/// not do itself.
fn camo_plan(host: &dyn ServerHost, stack: Option<&ItemStack>, camo: Option<&str>) -> Plan {
    let Some(stack) = stack else {
        return Plan::None;
    };
    let item_id = stack.item().id(host);
    if item_id == FRAMED_HAMMER {
        return match runtime::camo_action(Held::Hammer, camo) {
            CamoAction::Remove => Plan::Remove,
            _ => Plan::None,
        };
    }
    // Only a plain stack is stored: the frame keeps the camo's block and item ids, so a
    // stack carrying components or damage would be handed back as a plain one.
    if !runtime::may_be_camo(stack.components().is_some(), stack.damage()) {
        return Plan::None;
    }
    let Some(block_key) = host.item_block(stack.item()) else {
        return Plan::None;
    };
    let block_id = block_key.id(host);
    let held = Held::BlockItem {
        block: &block_id,
        item: &item_id,
    };
    // A camo set before this slice was a bare block id, which is still a state (the block's
    // default), so the rules test the block part of whatever is stored.
    let stored = camo.and_then(CamoState::parse);
    match runtime::camo_action(held, stored.as_ref().map(CamoState::block)) {
        CamoAction::Apply { item, .. } => Plan::Apply {
            state: default_state(host, block_key),
            item: item.to_owned(),
        },
        _ => Plan::None,
    }
}

/// The state a block item gives a frame: its block's default state, as the state string a client
/// draws (`BlockItem.getBlock().defaultBlockState()` in `applyCamo`). The bare block id when the
/// registry cannot serialise it, which is the same state for a block with no properties.
fn default_state(host: &dyn ServerHost, block: BlockKey) -> String {
    host.block_state_default(block)
        .and_then(|state| host.block_state_string(state))
        .unwrap_or_else(|| block.id(host))
}

/// The literal path of `/framedblocks camo`, without its arguments.
fn is_camo_command(literals: &[String]) -> bool {
    literals == ["framedblocks", "camo"]
}

/// The three integers `/framedblocks camo <x> <y> <z>` carries.
fn camo_position(arguments: &[CommandArgument]) -> Option<[i32; 3]> {
    let coordinate = |name: &str| {
        let argument = arguments.iter().rev().find(|entry| entry.name == name)?;
        match &argument.value {
            CommandArgumentValue::Integer(value) => Some(*value),
            _ => None,
        }
    };
    Some([coordinate("x")?, coordinate("y")?, coordinate("z")?])
}

/// The state `/framedblocks camo <x> <y> <z> <state>` was given, if any.
fn camo_state_argument(arguments: &[CommandArgument]) -> Option<String> {
    let argument = arguments.iter().rev().find(|entry| entry.name == "state")?;
    match &argument.value {
        CommandArgumentValue::String(value) => Some(value.clone()),
        _ => None,
    }
}

/// Stores `state` on the frame at `pos` and answers what the block entity now holds. The game
/// parses and serialises the state, so a property the block does not have (or a value it cannot
/// take) is refused here rather than stored for a client to draw wrong.
fn set_camo(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], state: &str) -> String {
    let [x, y, z] = pos;
    let Some(parsed) = host.block_state_id(state) else {
        return format!("framedblocks camo {x} {y} {z} = '{state}' is not a block state");
    };
    let Some(canonical) = host.block_state_string(parsed) else {
        return format!("framedblocks camo {x} {y} {z} = '{state}' is not a block state");
    };
    let Some(camo) = CamoState::parse(&canonical) else {
        return format!("framedblocks camo {x} {y} {z} = '{state}' is not a block state");
    };
    if !runtime::is_camo_block(camo.block()) {
        return format!(
            "framedblocks camo {x} {y} {z} = {} is never a camo",
            camo.block()
        );
    }
    let item = host
        .block_key(camo.block())
        .and_then(|block| host.block_item(block))
        .map(|item| item.id(host))
        .unwrap_or_default();
    if !write_camo(host, level, pos, &canonical, &item) {
        return format!("framedblocks camo {x} {y} {z} = no framed block there");
    }
    describe_camo(host, level, pos)
}

/// What `/framedblocks camo <x> <y> <z>` answers for the block entity at `pos`.
fn describe_camo(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> String {
    let [x, y, z] = pos;
    let Some(entity) = host.block_entity(level, pos) else {
        return format!("framedblocks camo {x} {y} {z} = no block entity");
    };
    let ours = entity
        .kind(host)
        .is_some_and(|kind| kind.id(host) == FRAMED_CUBE);
    if !ours {
        return format!("framedblocks camo {x} {y} {z} = another block entity");
    }
    let stored = entity.data(host).and_then(|data| {
        let (camo, item) = read_camo(&data);
        Some((camo?, item))
    });
    match stored {
        Some((camo, Some(item))) => format!(
            "framedblocks camo {x} {y} {z} = {} (item {item})",
            canonical_state(host, &camo)
        ),
        Some((camo, None)) => format!(
            "framedblocks camo {x} {y} {z} = {} (no item)",
            canonical_state(host, &camo)
        ),
        None => format!("framedblocks camo {x} {y} {z} = none"),
    }
}

/// The state as the game writes it back (`BlockStateParser.serialize`), or the stored string
/// when the registry no longer knows it: the readout must never replace a stored camo with a
/// different one.
fn canonical_state(host: &dyn ServerHost, stored: &str) -> String {
    host.block_state_id(stored)
        .and_then(|state| host.block_state_string(state))
        .unwrap_or_else(|| stored.to_owned())
}

/// Whether `state` is this port's frame.
fn is_frame(host: &dyn ServerHost, state: BlockState) -> bool {
    host.block_state_string(state).as_deref() == Some(FRAMED_CUBE)
}

/// The frame's camo block and camo item, from its block entity's saved data.
fn camo_at(
    host: &dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
) -> (Option<String>, Option<String>) {
    let Some(entity) = host.block_entity(level, pos) else {
        return (None, None);
    };
    let Some(data) = entity.data(host) else {
        return (None, None);
    };
    read_camo(&data)
}

/// The camo's state string (`minecraft:grass_block[snowy=true]`) and the item a break gives
/// back, from a saved compound.
fn read_camo(compound: &Compound) -> (Option<String>, Option<String>) {
    (
        string_tag(compound, CAMO_TAG),
        string_tag(compound, CAMO_ITEM_TAG),
    )
}

fn string_tag(compound: &Compound, key: &str) -> Option<String> {
    let Tag::String(value) = compound.get(&key.into())? else {
        return None;
    };
    let value = value.to_string_lossy();
    (!value.is_empty()).then_some(value)
}

/// Stores the camo state on the frame's block entity, through the vanilla write that also saves
/// and syncs it to clients (`/data merge block`'s path).
fn write_camo(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    state: &str,
    item: &str,
) -> bool {
    patch_camo(host, level, pos, state, item)
}

/// Clears the camo with empty values a client merges over the old ones.
fn clear_camo(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> bool {
    patch_camo(host, level, pos, "", "")
}

fn patch_camo(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    state: &str,
    item: &str,
) -> bool {
    let Some(entity) = host.block_entity(level, pos) else {
        return false;
    };
    let mut patch = Compound::new();
    patch.insert(CAMO_TAG.into(), Tag::String(state.into()));
    patch.insert(CAMO_ITEM_TAG.into(), Tag::String(item.into()));
    matches!(entity.merge_data(host, patch), Ok(true))
}

/// Takes one of `stack` from `hand`; creative is handled by the caller. An emptied hand becomes
/// empty, as `ItemStack.consume` leaves it.
fn consume_one(
    host: &mut dyn ServerHost,
    living: LivingRef,
    hand: Hand,
    stack: Option<&ItemStack>,
) {
    let Some(stack) = stack else {
        return;
    };
    let left = stack.count().saturating_sub(1);
    let held = if left == 0 {
        None
    } else {
        let mut held = stack.clone();
        if held.set_count(left).is_err() {
            return;
        }
        Some(held)
    };
    let _ = living.set_hand_item(host, hand, held);
}

/// `Inventory.add`: the camo item goes into a free main-inventory slot, and the camo stays when
/// there is none (`AbstractBlockCamoContainerFactory.removeCamo`).
fn give_back(host: &mut dyn ServerHost, player: PlayerRef, stack: ItemStack) -> bool {
    let Some(slots) = player.inventory_slots(&*host) else {
        return false;
    };
    let occupied: Vec<u32> = slots.into_iter().map(|(slot, _)| slot).collect();
    let Some(slot) = free_main_slot(&occupied) else {
        return false;
    };
    player.set_inventory_slot(host, slot, Some(stack)).is_ok()
}

/// The first empty main-inventory slot (`Inventory` slots 0 to 35), from the slots that hold a
/// stack.
fn free_main_slot(occupied: &[u32]) -> Option<u32> {
    (0..=MAIN_INVENTORY_END).find(|slot| !occupied.contains(slot))
}

/// The frame's own cube: the frame sprite on every face (upstream's `framed_cube` model).
fn draw_frame(draw: &mut dyn BlockEntityDrawContext) {
    let face = CubeFace::new(FRAME_SPRITE);
    draw.cube(
        [0.0, 0.0, 0.0],
        [16.0, 16.0, 16.0],
        [Some(face); 6],
        0x00FF_FFFF,
    );
}

/// The frames a player broke this tick, by position, with the camo item the loot roll gives
/// back. The block entity is gone by the time `loot_drops` runs, so the item is resolved while
/// the block entity is still readable.
static BROKEN_FRAMES: Mutex<BTreeMap<[i32; 3], ItemStack>> = Mutex::new(BTreeMap::new());

fn broken_frames() -> &'static Mutex<BTreeMap<[i32; 3], ItemStack>> {
    &BROKEN_FRAMES
}

fn stash_broken(pos: [i32; 3], stack: ItemStack) {
    broken_frames()
        .lock()
        .expect("the broken-frame stash lock is held for one map insert")
        .insert(pos, stack);
}

fn take_broken(pos: [i32; 3]) -> Option<ItemStack> {
    broken_frames()
        .lock()
        .expect("the broken-frame stash lock is held for one map removal")
        .remove(&pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_free_slot_is_the_first_one_no_stack_holds() {
        assert_eq!(free_main_slot(&[]), Some(0));
        assert_eq!(free_main_slot(&[0, 1, 2]), Some(3));
        assert_eq!(free_main_slot(&[0, 1, 3]), Some(2));
    }

    #[test]
    fn only_the_main_inventory_counts_for_a_free_slot() {
        // A full main inventory with equipment slots free still has no room: 36 to 42 are the
        // armour, off-hand and body slots, which the camo item never goes into.
        let full: Vec<u32> = (0..=MAIN_INVENTORY_END).collect();
        assert_eq!(free_main_slot(&full), None);
        let mut with_equipment = full.clone();
        with_equipment.extend(36..=42);
        assert_eq!(free_main_slot(&with_equipment), None);
        let mut one_free = full;
        one_free.retain(|slot| *slot != 35);
        assert_eq!(free_main_slot(&one_free), Some(35));
    }

    #[test]
    fn the_camo_command_is_the_three_integers_one() {
        let words = |literals: &[&str]| -> Vec<String> {
            literals.iter().map(|word| (*word).to_owned()).collect()
        };
        assert!(is_camo_command(&words(&["framedblocks", "camo"])));
        assert!(!is_camo_command(&words(&["framedblocks"])));
        assert!(!is_camo_command(&words(&["other", "camo"])));
        assert!(!is_camo_command(&words(&["framedblocks", "camo", "1"])));
    }

    #[test]
    fn the_camo_command_takes_a_state_after_the_position() {
        let string = |name: &str, value: &str| {
            CommandArgument::new(
                name.to_owned(),
                CommandArgumentValue::String(value.to_owned()),
            )
        };
        assert_eq!(
            camo_state_argument(&[string("state", "minecraft:grass_block[snowy=true]")]).as_deref(),
            Some("minecraft:grass_block[snowy=true]")
        );
        // The readout form carries no state, and a state argument of another type is not one.
        assert_eq!(camo_state_argument(&[]), None);
        let integer = |name: &str, value| CommandArgument::new(name.to_owned(), value);
        assert_eq!(
            camo_state_argument(&[integer("state", CommandArgumentValue::Integer(1))]),
            None
        );
    }

    #[test]
    fn the_camo_position_reads_three_integers() {
        let position = |arguments: &[CommandArgument]| camo_position(arguments);
        let integer = |name: &str, value| CommandArgument::new(name.to_owned(), value);
        assert_eq!(
            position(&[
                integer("x", CommandArgumentValue::Integer(100)),
                integer("y", CommandArgumentValue::Integer(200)),
                integer("z", CommandArgumentValue::Integer(-3)),
            ]),
            Some([100, 200, -3])
        );
        // A missing or wrongly typed argument is not the command's position.
        assert_eq!(
            position(&[
                integer("x", CommandArgumentValue::Integer(1)),
                integer("y", CommandArgumentValue::Integer(2)),
            ]),
            None
        );
        assert_eq!(
            position(&[
                integer("x", CommandArgumentValue::Integer(1)),
                integer("y", CommandArgumentValue::String("2".to_owned())),
                integer("z", CommandArgumentValue::Integer(3)),
            ]),
            None
        );
    }
}
