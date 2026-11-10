// Runs `/framedblocks camo <x> <y> <z>` when a player types it: the camo
// stored on the frame's block entity, or none (API 8).
fn perform_command(
    game: &mut crate::game::Game<'_>,
    player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    crate::game::framedblocks_state::on_perform_command(game, player, command, ran);
}
