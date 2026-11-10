// The camo item drops beside the frame (`FramedBlockEntity.addCamoDrops`,
// `LootTableEvents.MODIFY_DROPS`, API 75).
fn observe_loot_drops(
    game: &mut crate::game::Game<'_>,
    context: &NativeLootDrops<'_>,
    drops: &mut Vec<ItemStack>,
) {
    crate::game::framedblocks_state::on_loot_drops(game, context, drops);
}
