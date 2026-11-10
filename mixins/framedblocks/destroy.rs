// A player break of a frame: remember its camo before the block entity goes,
// for the loot roll in the same tick (API 14). The break itself is unchanged.
fn observe_destroy(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
) {
    crate::game::framedblocks_state::on_destroy(game, pos, state);
}
