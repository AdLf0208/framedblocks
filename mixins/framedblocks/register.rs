// Declares `/framedblocks camo <x> <y> <z>`, the server's readout of a frame's
// stored camo (API 8).
fn register_commands(commands: &mut Vec<NativeCommand>) {
    crate::game::framedblocks_state::on_register_commands(commands);
}
