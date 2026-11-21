// FramedBlocks for Horizon — the camo's block state, as the frame stores it (pure).
//
// Upstream's camo container holds a `BlockState` (`FramedBlockEntity.camoContainer`,
// `AbstractBlockCamoContainerFactory.applyCamo` reads `BlockItem.getBlock().defaultBlockState()`),
// and the client draws that state. This port stores what `/setblock` takes
// (`BlockStateParser.serialize`): the block id, then its properties in name order,
// `minecraft:grass_block[snowy=true]`; a block with no properties is its bare id. The game
// resolves and canonicalises the string (`ServerHost::block_state_id` / `block_state_string`);
// this module only splits a stored state into what the camo rules test and what
// `BlockEntityDrawContext::block` draws with.
//
// A camo with properties is what a command or a blueprint supplies; a block item still gives its
// block's default state, exactly as the item path always has.

/// A stored camo state: the block it names and the properties it carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CamoState<'a> {
    block: &'a str,
    properties: Vec<(&'a str, &'a str)>,
}

impl<'a> CamoState<'a> {
    /// Splits a state in `/setblock` form into its block id and its properties.
    ///
    /// `minecraft:gold_block` is one with no properties; `minecraft:oak_stairs[facing=east]`
    /// carries the property the camo was given. None for anything a canonical state string
    /// cannot be: an empty block, an unclosed or empty property list, a property without a name
    /// or a value, or anything after the list.
    pub fn parse(state: &'a str) -> Option<Self> {
        let (block, properties) = match state.split_once('[') {
            Some((block, rest)) => {
                let list = rest.strip_suffix(']')?;
                // A second list, or anything else after the bracket, is not a state.
                if list.contains('[') || list.contains(']') || list.is_empty() {
                    return None;
                }
                (block, Some(list))
            }
            None => (state, None),
        };
        if !is_block_name(block) {
            return None;
        }
        let properties = match properties {
            None => Vec::new(),
            Some(list) => list.split(',').map(property).collect::<Option<_>>()?,
        };
        Some(Self { block, properties })
    }

    /// The block the camo names (`minecraft:grass_block`), which is what the camo rules test.
    pub fn block(&self) -> &'a str {
        self.block
    }

    /// The properties the camo was given, in the order the state writes them.
    pub fn properties(&self) -> &[(&'a str, &'a str)] {
        &self.properties
    }
}

/// Whether `name` is a block id: not empty and no character a state string reserves.
fn is_block_name(name: &str) -> bool {
    !name.is_empty()
        && !name
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '[' | ']' | ',' | '='))
}

/// One `name=value` of a property list.
fn property(text: &str) -> Option<(&str, &str)> {
    let (name, value) = text.split_once('=')?;
    let shaped = |part: &str| !part.is_empty() && !part.chars().any(|c| c.is_whitespace());
    (shaped(name) && shaped(value) && !value.contains('=')).then_some((name, value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime;

    #[test]
    fn a_bare_block_id_has_no_properties() {
        let state = CamoState::parse("minecraft:gold_block").expect("a block id is a state");
        assert_eq!(state.block(), "minecraft:gold_block");
        assert_eq!(state.properties().to_vec(), Vec::new());
    }

    #[test]
    fn the_properties_after_a_block_id_are_kept_in_order() {
        let state = CamoState::parse("minecraft:oak_stairs[facing=east,half=top]")
            .expect("a state with two properties parses");
        assert_eq!(state.block(), "minecraft:oak_stairs");
        assert_eq!(
            state.properties().to_vec(),
            vec![("facing", "east"), ("half", "top")]
        );
    }

    #[test]
    fn a_state_without_a_block_is_refused() {
        assert_eq!(CamoState::parse(""), None, "an empty state names no block");
        assert_eq!(
            CamoState::parse("[snowy=true]"),
            None,
            "a property list needs a block before it"
        );
        assert_eq!(
            CamoState::parse("minecraft:grass_block[]"),
            None,
            "a canonical state never writes empty brackets"
        );
    }

    #[test]
    fn an_unclosed_property_list_is_refused() {
        assert_eq!(CamoState::parse("minecraft:grass_block[snowy=true"), None);
    }

    #[test]
    fn a_property_must_have_a_name_and_a_value() {
        assert_eq!(CamoState::parse("minecraft:stone[snowy]"), None);
        assert_eq!(CamoState::parse("minecraft:stone[snowy=]"), None);
        assert_eq!(CamoState::parse("minecraft:stone[=true]"), None);
    }

    #[test]
    fn an_empty_property_is_refused() {
        assert_eq!(CamoState::parse("minecraft:stone[a=b,]"), None);
        assert_eq!(CamoState::parse("minecraft:stone[,a=b]"), None);
        assert_eq!(CamoState::parse("minecraft:stone[a=b,,c=d]"), None);
    }

    #[test]
    fn text_after_the_property_list_is_refused() {
        assert_eq!(CamoState::parse("minecraft:stone[snowy=true]x"), None);
        assert_eq!(CamoState::parse("minecraft:stone[a=b][c=d]"), None);
        assert_eq!(CamoState::parse("minecraft:stone[a=b] c=d"), None);
    }

    #[test]
    fn the_block_part_is_what_the_camo_rules_see() {
        // A framed block is never a camo, whatever state it names (`findCamoFactory`).
        let frame = CamoState::parse("framedblocks:framed_cube").expect("the frame parses");
        assert!(!runtime::is_camo_block(frame.block()));
        let stone = CamoState::parse("minecraft:stone[snowy=true]").expect("stone parses");
        assert!(runtime::is_camo_block(stone.block()));
    }
}
