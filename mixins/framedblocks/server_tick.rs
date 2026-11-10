// The server tick's end: resend the inventories a camo use changed, and forget
// a break whose loot never rolled (API 1).
fn observe_server_tick(game: &mut crate::game::Game<'_>, phase: ServerTickPhase) {
    crate::game::framedblocks_state::on_server_tick(game, phase);
}
