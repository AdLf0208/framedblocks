// A right click on a frame: a block item sets the camo, the framed hammer
// takes it off (`FramedBlockEntity.handleInteraction`, API 13).
fn observe_use_item_on(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    pos: [i32; 3],
    claimed: &mut bool,
) {
    crate::game::framedblocks_state::on_use_item_on(game, stack, pos, claimed);
}
