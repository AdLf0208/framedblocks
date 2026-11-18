// FramedBlocks for Horizon — the camo decision model (pure).
//
// Ported from FramedBlocks 11.4.0 (XFactHD/FramedBlocks, LGPL-3.0-only) for
// Minecraft Java 26.1.2 NeoForge; the ATM10 pack ships 10.6.1 for 1.21.1.
// Upstream source was read for behaviour only; this is a clean-room Rust
// rewrite. LOCAL-ONLY until the maintainer approves publishing.
//
// A framed block stores a camo: another block's state, whose model the client
// draws in the frame's place (`FramedBlockEntity.camoContainer`,
// `AbstractFramedBlock.getAppearance`). This slice ships one full-cube frame
// (`framedblocks:framed_cube`) and block camos:
//
// - a right click with a block item on an empty frame takes the item's block
//   and consumes one in survival (`FramedBlockEntity.handleInteraction` ->
//   `AbstractBlockCamoContainerFactory.applyCamo`; the state is
//   `BlockItem.getBlock().defaultBlockState()`, so a plain block item is a
//   default-state camo);
// - a right click with the framed hammer on a set camo removes it and gives
//   the camo item back (`CamoContainerFactories.isValidRemovalTool` ->
//   `removeCamo`);
// - a framed block is never a camo (`findCamoFactory` returns null for an
//   `IFramedBlock` item), and a block item on an occupied frame does nothing;
// - a break drops the camo item beside the frame (`getCamoDrops` ->
//   `addAdditionalDrops`).
//
// The frame's own model is empty: the client draws the camo block's model, or
// the frame's texture when no camo is set (`native_block_entity_submit`).

/// The one frame this slice ships (`FBContent.BLOCK_FRAMED_CUBE`).
pub const FRAMED_CUBE: &str = "framedblocks:framed_cube";
/// The hammer that removes a camo (`FBContent.ITEM_FRAMED_HAMMER`).
pub const FRAMED_HAMMER: &str = "framedblocks:framed_hammer";
/// Air is never a camo (`AbstractBlockCamoContainerFactory.validateCamo`).
pub const AIR: &str = "minecraft:air";
/// The frame texture the client draws when no camo is set
/// (`textures/block/framed_block.png`).
pub const FRAME_SPRITE: &str = "framedblocks:block/framed_block";
/// The block entity compound entry holding the camo block's id.
pub const CAMO_TAG: &str = "Camo";
/// The block entity compound entry holding the item a break gives back.
pub const CAMO_ITEM_TAG: &str = "CamoItem";

/// What a player holds when right-clicking a framed block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held<'a> {
    /// A block item: the block it places and its own item id.
    Block { block: &'a str, item: &'a str },
    /// The framed hammer (`FramedConstants.Objects.FRAMED_HAMMER`).
    Hammer,
    /// Anything else, an empty hand included.
    Other,
}

/// What the right click does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CamoAction<'a> {
    /// Put `block` on the frame and remember `item` for the drop.
    Apply { block: &'a str, item: &'a str },
    /// Take the camo off and give its item back.
    Remove,
    /// Leave the click to the game (it places the item, or does nothing).
    None,
}

/// Whether `block` can be a camo: a block item's block, never air and never
/// another framed block.
pub fn is_camo_block(block: &str) -> bool {
    !block.is_empty() && block != AIR && block != FRAMED_CUBE
}

/// What a right click on a framed block does, from the held item and the camo
/// already set on the frame.
pub fn camo_action<'a>(held: Held<'a>, camo: Option<&str>) -> CamoAction<'a> {
    match (held, camo.filter(|block| is_camo_block(block))) {
        (Held::Block { block, item }, None) if is_camo_block(block) => {
            CamoAction::Apply { block, item }
        }
        (Held::Hammer, Some(_)) => CamoAction::Remove,
        _ => CamoAction::None,
    }
}

/// What the client draws in the frame's place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Render<'a> {
    /// The camo block's default-state model (`native_draw_block`).
    Camo(&'a str),
    /// The frame's own texture (`native_draw_cube`).
    Frame,
}

/// The model the client draws for `camo`.
pub fn render(camo: Option<&str>) -> Render<'_> {
    match camo.filter(|block| is_camo_block(block)) {
        Some(block) => Render::Camo(block),
        None => Render::Frame,
    }
}

/// The item a break drops for the camo on the frame: the item that applied
/// it (`FramedBlockEntity.addCamoDrops` -> `CamoContainerHelper.dropCamo`).
pub fn camo_drop(camo_item: Option<&str>) -> Option<&str> {
    camo_item.filter(|item| !item.is_empty())
}

/// Whether applying or removing a camo takes one from the player: upstream
/// does in survival and not in creative (`applyCamo`/`removeCamo`).
pub fn consumes(creative: bool) -> bool {
    !creative
}

/// What `/framedblocks camo <x> <y> <z>` asks for: the block position whose
/// camo to report (`native_feedback`), for the e2e and for a player.
pub fn parse_camo_command(line: &str) -> Option<[i32; 3]> {
    let words: Vec<&str> = line.split_whitespace().collect();
    match words.as_slice() {
        ["framedblocks", "camo", x, y, z] => {
            Some([x.parse().ok()?, y.parse().ok()?, z.parse().ok()?])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STONE: Held<'static> = Held::Block {
        block: "minecraft:stone",
        item: "minecraft:stone",
    };

    #[test]
    fn a_block_item_on_an_empty_frame_applies_its_block() {
        assert_eq!(
            camo_action(STONE, None),
            CamoAction::Apply {
                block: "minecraft:stone",
                item: "minecraft:stone"
            }
        );
    }

    #[test]
    fn a_framed_block_is_never_a_camo() {
        let frame = Held::Block {
            block: FRAMED_CUBE,
            item: FRAMED_CUBE,
        };
        assert_eq!(camo_action(frame, None), CamoAction::None);
        assert!(!is_camo_block(FRAMED_CUBE));
    }

    #[test]
    fn air_is_never_a_camo() {
        let air = Held::Block {
            block: AIR,
            item: AIR,
        };
        assert_eq!(camo_action(air, None), CamoAction::None);
        assert!(!is_camo_block(AIR));
    }

    #[test]
    fn a_block_item_does_not_replace_a_set_camo() {
        assert_eq!(
            camo_action(STONE, Some("minecraft:gold_block")),
            CamoAction::None
        );
    }

    #[test]
    fn the_hammer_removes_a_set_camo() {
        assert_eq!(
            camo_action(Held::Hammer, Some("minecraft:gold_block")),
            CamoAction::Remove
        );
    }

    #[test]
    fn the_hammer_on_an_empty_frame_does_nothing() {
        assert_eq!(camo_action(Held::Hammer, None), CamoAction::None);
    }

    #[test]
    fn an_other_item_does_nothing() {
        assert_eq!(camo_action(Held::Other, None), CamoAction::None);
        assert_eq!(
            camo_action(Held::Other, Some("minecraft:stone")),
            CamoAction::None
        );
    }

    #[test]
    fn the_client_draws_the_camo_block_or_the_frame() {
        assert_eq!(
            render(Some("minecraft:gold_block")),
            Render::Camo("minecraft:gold_block")
        );
        assert_eq!(render(None), Render::Frame);
        assert_eq!(render(Some("")), Render::Frame);
    }

    #[test]
    fn a_break_drops_the_camo_item_only_when_one_was_applied() {
        assert_eq!(
            camo_drop(Some("minecraft:gold_block")),
            Some("minecraft:gold_block")
        );
        assert_eq!(camo_drop(Some("")), None);
        assert_eq!(camo_drop(None), None);
    }

    #[test]
    fn only_survival_consumes_the_camo_item() {
        assert!(consumes(false));
        assert!(!consumes(true));
    }

    #[test]
    fn the_camo_command_takes_three_integers() {
        assert_eq!(
            parse_camo_command("framedblocks camo 100 200 -3"),
            Some([100, 200, -3])
        );
        assert_eq!(
            parse_camo_command("  framedblocks   camo  0 64 0 "),
            Some([0, 64, 0])
        );
    }

    #[test]
    fn other_command_lines_are_not_the_camo_command() {
        assert_eq!(parse_camo_command("framedblocks camo 1 2"), None);
        assert_eq!(parse_camo_command("framedblocks camo 1 2 3 4"), None);
        assert_eq!(parse_camo_command("framedblocks camo a b c"), None);
        assert_eq!(parse_camo_command("framedblocks other 1 2 3"), None);
        assert_eq!(parse_camo_command("camo 1 2 3"), None);
    }
}
