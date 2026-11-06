// A player breaking a storage barrel: its Storage Upgrades go with the barrel (barrel.rs's drop
// carries how many, as of its last tick, and placing it puts them back), so they are taken out of
// its upgrade slot here rather than spilling. No side reaches that slot, so only mixin code can.
// Creative breaks drop nothing, so a barrel broken there spills as before.
fn horizon_chests_core_keep_upgrades(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    if game.creative {
        return;
    }
    let block = {
        let host: &dyn horizon_client::mod_api::Host = &*game;
        host.block(pos)
    };
    // A Compacting Drawer's outputs only show its count, which goes with the drop too.
    let cleared: &[usize] = match block.as_str() {
        "horizon_chests:storage_barrel" => &[2],
        "horizon_chests:compacting_drawer" => &[1, 2, 3, 4],
        _ => return,
    };
    game.native_with_mod_block_entity(pos, |be| {
        for &i in cleared {
            if let Some(slot) = be.items.get_mut(i) {
                *slot = None;
            }
        }
    });
}
