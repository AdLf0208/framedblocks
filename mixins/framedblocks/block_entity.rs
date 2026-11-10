// The framed cube declares a block entity for its camo (`native_block_entity`,
// API 129: any template, not only a chest's or a furnace's).
fn observe_block_entity(
    block: &str,
    entity: &mut Option<(String, horizon_world::nbt::Compound)>,
) {
    crate::game::framedblocks_state::on_block_entity(block, entity);
}
