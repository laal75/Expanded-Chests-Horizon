//! Chests that keep what they hold when broken, as a shulker box does. The game spills a
//! container's slots on its own when the block goes, so when a player starts breaking one
//! (destroy_block) its items are taken out first; the chest that drops carries them in its
//! custom data (loot_drops), and placing it puts them back (place_block).
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use horizon_client::mod_api::{
    Direction, Host, ItemStack, LootDropsEvent, PlaceBlockEvent, Tag, Text, TooltipEvent,
};

/// The key a dropped chest carries its items under, in `minecraft:custom_data`.
const KEY_ITEMS: &str = "chest_items";

/// What a player's break took out of a chest, by level and position, until its drop takes it.
fn taken() -> &'static Mutex<HashMap<(String, [i32; 3]), Vec<Option<ItemStack>>>> {
    static MAP: OnceLock<Mutex<HashMap<(String, [i32; 3]), Vec<Option<ItemStack>>>>> = OnceLock::new();
    MAP.get_or_init(Default::default)
}

/// A player in survival or adventure breaks `block` at `pos`: empty it, so nothing spills.
/// Creative breaks drop nothing, so a chest broken there spills as before.
pub(crate) fn destroy_block(host: &mut dyn Host, pos: [i32; 3], slots: usize) {
    if host.acting_player().is_none_or(|p| p.game_mode == 1) {
        return;
    }
    let dimension = host.dimension();
    let mut items = vec![None; slots];
    let mut any = false;
    for (slot, cell) in items.iter_mut().enumerate() {
        if let Some(stack) = host.extract_item(dimension, pos, Direction::Up, slot, u32::MAX) {
            *cell = Some(stack);
            any = true;
        }
    }
    if any {
        taken().lock().unwrap().insert((dimension.to_owned(), pos), items);
    }
}

/// The chest that drops takes what was in it.
pub(crate) fn loot_drops(event: &mut LootDropsEvent<'_>, is_chest: impl Fn(&str) -> bool) {
    let Some((pos, _)) = event.context.block else {
        return;
    };
    let key = (event.context.dimension.to_owned(), pos);
    let Some(items) = taken().lock().unwrap().remove(&key) else {
        return;
    };
    match event.drops.iter_mut().find(|s| is_chest(&s.item)) {
        Some(drop) => {
            let mut data = match drop.data().and_then(|d| d.other("minecraft:custom_data")) {
                Some(Tag::Compound(c)) => c.clone(),
                _ => Default::default(),
            };
            data.insert(KEY_ITEMS.into(), horizon_client::save::encode_items(&items));
            drop.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(data)));
        }
        // No chest dropped (an explosion that left none): the items drop loose.
        None => event.drops.extend(items.into_iter().flatten()),
    }
}

/// A placed chest that held items gets them back; what doesn't fit goes to the player.
pub(crate) fn place_block(host: &mut dyn Host, event: &mut PlaceBlockEvent<'_>) {
    let Some(items) = carried(event.stack) else {
        return;
    };
    let player = host.acting_player();
    for stack in items.into_iter().flatten() {
        if let Some(back) = host.insert_item(event.pos, Direction::Up, stack)
            && back.count > 0
            && let Some(p) = &player
        {
            let _ = host.give_stack(p.uuid, back);
        }
    }
}

pub(crate) fn tooltip(event: &mut TooltipEvent<'_>) {
    let Some(items) = carried(event.stack) else {
        return;
    };
    let (stacks, count) = items.iter().flatten().fold((0, 0), |(n, c), s| (n + 1, c + s.count));
    event.lines.push((Text::literal(format!("{count} items in {stacks} stacks")), 0xAA_AAAA));
}

/// The items a chest item carries, from its custom data.
fn carried(stack: &ItemStack) -> Option<Vec<Option<ItemStack>>> {
    let Tag::Compound(data) = stack.data()?.other("minecraft:custom_data")? else {
        return None;
    };
    let tag = get(data, KEY_ITEMS)?;
    // Slot numbers are a byte, so 256 covers every chest.
    let items = horizon_client::save::decode_items(Some(tag), 256);
    items.iter().any(Option::is_some).then_some(items)
}

/// `map[name]`, with the key type taken from the map (the game's NBT string isn't exported).
fn get<'m, K: Ord + for<'a> From<&'a str>, V>(map: &'m std::collections::BTreeMap<K, V>, name: &str) -> Option<&'m V> {
    map.get(&K::from(name))
}
