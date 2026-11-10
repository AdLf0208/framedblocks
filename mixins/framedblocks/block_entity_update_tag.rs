// Clients get the frame's camo (`BlockEntity.getUpdateTag`): the camo block's
// id and the item a break gives back, or empty strings after a removal.
fn observe_block_entity_update_tag(
    entity: &horizon_client::containers::ModBlockEntity,
    tag: &mut Option<horizon_world::nbt::Compound>,
) {
    crate::game::framedblocks_state::on_block_entity_update_tag(entity, tag);
}
