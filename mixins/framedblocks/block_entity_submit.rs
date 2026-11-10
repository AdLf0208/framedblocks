// The client draws the camo block's model in the frame's place, or the frame's
// own texture when no camo is set (`BlockEntityRenderer.submit`, API 71).
fn observe_block_entity_submit(
    game: &crate::game::Game<'_>,
    block: &str,
    entity: &ModBlockEntity,
    draw: &mut BlockEntityDraw,
) {
    crate::game::framedblocks_state::on_block_entity_submit(game, block, entity, draw);
}
