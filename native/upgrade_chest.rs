// Upgrading a chest where it stands: sneak and right-click a chest holding the next tier's Chest
// Upgrade (a single vanilla chest takes an Iron Chest Upgrade; an Iron Chest a Gold Chest
// Upgrade; a Gold Chest a Diamond Chest Upgrade; a Diamond Chest an Obsidian Chest Upgrade). The
// upgrade is used up, the chest becomes the next tier facing the same way, and everything it
// held goes back in. Setting a block from its id is `Game`'s, which only mixin code reaches.
fn horizon_chests_core_upgrade_chest(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::mod_api::ItemStack,
    pos: [i32; 3],
    claimed: &mut bool,
) {
    use horizon_client::mod_api::{Direction, Host};
    if *claimed || game.remote.is_some() || !game.player.as_ref().is_some_and(|p| p.shift_key_down) {
        return;
    }
    let (block, facing, single) = {
        let host: &dyn Host = &*game;
        let block = host.block(pos);
        let single = host.block_property(pos, "type").is_none_or(|t| t == "single");
        (block, host.block_property(pos, "facing"), single)
    };
    // The next tier, and what it takes.
    let (next, upgrade) = match block.as_str() {
        "minecraft:chest" if single => ("horizon_chests:iron_chest", "horizon_chests:iron_chest_upgrade"),
        "horizon_chests:iron_chest" => ("horizon_chests:gold_chest", "horizon_chests:gold_chest_upgrade"),
        "horizon_chests:gold_chest" => ("horizon_chests:diamond_chest", "horizon_chests:diamond_chest_upgrade"),
        "horizon_chests:diamond_chest" => ("horizon_chests:obsidian_chest", "horizon_chests:obsidian_chest_upgrade"),
        _ => return,
    };
    // Held: the upgrade for this chest.
    if &*stack.item != upgrade {
        return;
    }
    let cost: &[(&str, u32)] = &[(upgrade, 1)];
    let Some(state) = game.pack.assets.registry.default_state(next) else {
        return;
    };
    let enough = {
        let host: &mut dyn Host = &mut *game;
        cost.iter().all(|(item, n)| host.count_items(item) >= *n)
    };
    if !enough {
        return;
    }
    *claimed = true;
    let dimension = game.dim.id().to_string();
    // Out with what it holds, so nothing spills when the block changes.
    let mut held = Vec::new();
    if let Some(slots) = game.native_container_slots(&dimension, pos, Direction::Up) {
        for slot in slots {
            if slot.stack.is_some() {
                if let Some(s) = game.native_extract_item(&dimension, pos, Direction::Up, slot.slot, u32::MAX) {
                    held.push(s);
                }
            }
        }
    }
    if !game.native_set_block(&dimension, pos, state, 3) {
        // It stayed as it was: everything goes back.
        let host: &mut dyn Host = &mut *game;
        for s in held {
            if let Some(back) = host.insert_item(pos, Direction::Up, s) {
                if let Some(p) = host.acting_player() {
                    let _ = host.give_stack(p.uuid, back);
                }
            }
        }
        return;
    }
    let host: &mut dyn Host = &mut *game;
    if let Some(f) = facing {
        let _ = host.set_block_property(pos, "facing", &f);
    }
    for (item, n) in cost {
        host.take_items(item, *n);
    }
    let player = host.acting_player();
    for s in held {
        if let Some(back) = host.insert_item(pos, Direction::Up, s) {
            if let Some(p) = &player {
                let _ = host.give_stack(p.uuid, back);
            }
        }
    }
}
