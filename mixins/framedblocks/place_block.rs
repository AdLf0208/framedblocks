// A frame placed with a block item in the off hand takes it as its camo
// (`AbstractFramedBlock.setPlacedBy` -> `tryApplyCamoImmediately`, API 14).
fn observe_place_block(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
) {
    crate::game::framedblocks_state::on_place_block(game, pos, state);
}
