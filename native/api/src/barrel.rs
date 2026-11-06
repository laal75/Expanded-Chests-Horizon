//! The storage barrel: one kind of item, kept as a count in the block entity rather than in
//! slots (a slot holds 99 at most). It holds 32 stacks, doubled by each Storage Upgrade, to 32768
//! stacks with ten. Three slots face the world: what goes into the input is added to the count,
//! and the output is kept topped up from it, so hoppers and tubes fill and empty it as they do a
//! chest. Broken, it drops as a barrel that still holds its count (loot_drops), and placing that
//! barrel gives the count back (place_block), with its Storage Upgrades (keep_upgrades.rs takes
//! them out of the slot as it breaks, so they don't spill).
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use horizon_client::mod_api::{
    BlockEntityTickEvent, Compound, Direction, Flow, Host, HudBatch, ItemStack, LootDropsEvent,
    MenuSlotRulesEvent, ModBlockEntity, ModContainer, PlaceBlockEvent, ScreenDrawEvent, Tag, Text,
    TooltipEvent, UseItemOnEvent,
};

pub(crate) const BARREL: &str = "horizon_chests:storage_barrel";
const UPGRADE: &str = "horizon_chests:storage_upgrade";

const INPUT: usize = 0;
const OUTPUT: usize = 1;
const UPGRADES: usize = 2;
const SLOTS: usize = 3;

const BASE_STACKS: u32 = 32;
const MAX_UPGRADES: u32 = 10;

// Data slots (16-bit on the wire): the count in two halves, its upgrades (32768 stacks wouldn't fit
// a slot, so the screen works out its room from them), and the item's stack size.
const D_COUNT_LOW: usize = 0;
const D_COUNT_HIGH: usize = 1;
const D_UPGRADES: usize = 2;
const D_STACK_SIZE: usize = 3;
const DATA_SLOTS: usize = 4;

/// The `minecraft:custom_data` keys a dropped barrel carries its contents under.
const KEY_ITEM: &str = "barrel_item";
const KEY_COUNT: &str = "barrel_count";
const KEY_UPGRADES: &str = "barrel_upgrades";

pub(crate) fn container() -> ModContainer {
    ModContainer {
        slots: SLOTS,
        faces: std::array::from_fn(|_| vec![INPUT, OUTPUT]),
        inputs: vec![INPUT],
        outputs: vec![OUTPUT],
        menu: Some(BARREL.to_owned()),
    }
}

pub(crate) fn screen_slots() -> Vec<Option<[f32; 2]>> {
    vec![Some([44.0, 35.0]), Some([116.0, 35.0]), Some([152.0, 62.0])]
}

/// Each barrel's item and count, by level and position, as of its last tick: what a broken one
/// drops, since the loot hook can't see its block entity.
fn contents() -> &'static Mutex<HashMap<(&'static str, [i32; 3]), (String, u32, u32)>> {
    static MAP: OnceLock<Mutex<HashMap<(&'static str, [i32; 3]), (String, u32, u32)>>> = OnceLock::new();
    MAP.get_or_init(Default::default)
}

/// The item the barrel at `pos` in level `dimension` holds, as of its last tick; None for an
/// empty barrel or none there. Other mods read it through `crate::barrel_item`.
pub(crate) fn stored_item(dimension: &str, pos: [i32; 3]) -> Option<String> {
    let map = contents().lock().ok()?;
    map.iter()
        .find(|((d, p), _)| *d == dimension && *p == pos)
        // The count leaves out what sits in the output, and a barrel of a stack or less keeps it
        // all there: the item, cleared once both are empty, is what says it holds something.
        .and_then(|(_, (item, ..))| (!item.is_empty()).then(|| item.clone()))
}

/// What a barrel placed this tick held as an item, for its first tick to take in.
fn placed() -> &'static Mutex<HashMap<(&'static str, [i32; 3]), (String, u32, u32)>> {
    static MAP: OnceLock<Mutex<HashMap<(&'static str, [i32; 3]), (String, u32, u32)>>> = OnceLock::new();
    MAP.get_or_init(Default::default)
}

/// Only plain items go in: the count can't keep a stack's name, damage or enchantments.
fn plain(stack: &ItemStack) -> bool {
    stack.damage == 0 && stack.data().is_none_or(|d| d.is_empty())
}

fn stored(entity: &ModBlockEntity) -> (String, u32) {
    let item = match get(&entity.nbt, "item") {
        Some(Tag::String(s)) => s.to_string_lossy(),
        _ => String::new(),
    };
    let count = match get(&entity.nbt, "count") {
        Some(Tag::Int(n)) => (*n).max(0) as u32,
        _ => 0,
    };
    (item, count)
}

pub(crate) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
    let pos = event.pos;
    let dimension = host.dimension();
    let (mut item, mut count) = stored(event.entity);
    let carried_upgrades = match placed().lock().unwrap().remove(&(dimension, pos)) {
        Some((i, c, u)) => {
            (item, count) = (i, c);
            u
        }
        None => 0,
    };
    let be = &mut **event.entity;
    if be.items.len() < SLOTS {
        be.items.resize(SLOTS, None);
    }
    if carried_upgrades > 0 && be.items[UPGRADES].is_none() {
        be.items[UPGRADES] = Some(ItemStack::new(UPGRADE, carried_upgrades.min(MAX_UPGRADES)));
    }
    let upgrades = be.items[UPGRADES]
        .as_ref()
        .filter(|s| &*s.item == UPGRADE)
        .map_or(0, |s| s.count.min(MAX_UPGRADES));
    let stacks = BASE_STACKS << upgrades;
    let out_count = be.items[OUTPUT].as_ref().map_or(0, |s| s.count);
    if count == 0 && out_count == 0 {
        item.clear();
    }
    // Take in what the input holds, when it is this barrel's item and there is room.
    if let Some(input) = &mut be.items[INPUT]
        && plain(input)
        && (item.is_empty() || *input.item == *item)
    {
        let stack_size = host.max_stack_size(&input.item).unwrap_or(64);
        let room = (stacks * stack_size).saturating_sub(count + out_count);
        let took = room.min(input.count);
        if took > 0 {
            item = input.item.to_string();
            count += took;
            input.count -= took;
        }
        if input.count == 0 {
            be.items[INPUT] = None;
        }
    }
    let stack_size = if item.is_empty() { 64 } else { host.max_stack_size(&item).unwrap_or(64) };
    // Keep the output full from the count.
    if !item.is_empty() {
        let out = be.items[OUTPUT].get_or_insert_with(|| ItemStack::new(&item, 0));
        if *out.item == *item {
            let add = stack_size.saturating_sub(out.count).min(count);
            out.count += add;
            count -= add;
        }
        if out.count == 0 {
            be.items[OUTPUT] = None;
        }
    }
    let out_count = be.items[OUTPUT].as_ref().map_or(0, |s| s.count);
    be.nbt.insert("item".into(), Tag::String(item.as_str().into()));
    be.nbt.insert("count".into(), Tag::Int(count as i32));
    let total = count + out_count;
    be.data.resize(DATA_SLOTS, 0);
    be.data[D_COUNT_LOW] = (total & 0x7FFF) as i32;
    be.data[D_COUNT_HIGH] = (total >> 15) as i32;
    be.data[D_UPGRADES] = upgrades as i32;
    be.data[D_STACK_SIZE] = stack_size as i32;
    contents().lock().unwrap().insert((dimension, pos), (item, count, upgrades));
}

/// The input takes plain items, the upgrade slot Storage Upgrades (ten at most).
pub(crate) fn slot_rule(event: &mut MenuSlotRulesEvent<'_>) {
    match event.slot {
        INPUT => event.may_place = plain(event.stack) && &*event.stack.item != UPGRADE,
        UPGRADES => {
            event.may_place = &*event.stack.item == UPGRADE;
            event.max_stack = MAX_UPGRADES;
        }
        _ => {}
    }
}

/// A sneaking right click with an item (an ordinary one opens the screen) puts the held stack
/// in, as much as the input takes at once.
pub(crate) fn use_item_on(host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
    if host.block(event.pos) != BARREL || !plain(event.stack) || &*event.stack.item == UPGRADE {
        return Flow::Continue;
    }
    let Some(player) = host.acting_player() else {
        return Flow::Continue;
    };
    let item = event.stack.item.to_string();
    let took = host.take_items(&item, event.stack.count);
    if took > 0
        && let Some(back) = host.insert_item(event.pos, Direction::Up, ItemStack::new(&item, took))
        && back.count > 0
    {
        let _ = host.give_stack(player.uuid, back);
    }
    Flow::Cancel
}

/// A broken barrel's drop carries what it held.
pub(crate) fn loot_drops(event: &mut LootDropsEvent<'_>) {
    let Some((pos, _)) = event.context.block else {
        return;
    };
    let Some(drop) = event.drops.iter_mut().find(|s| &*s.item == BARREL) else {
        return;
    };
    let key = (dimension_key(event.context.dimension), pos);
    let Some((item, count, upgrades)) = contents().lock().unwrap().remove(&key) else {
        return;
    };
    let holds = !item.is_empty() && count > 0;
    if !holds && upgrades == 0 {
        return;
    }
    let mut data = Compound::new();
    if holds {
        data.insert(KEY_ITEM.into(), Tag::String(item.as_str().into()));
        data.insert(KEY_COUNT.into(), Tag::Int(count as i32));
    }
    if upgrades > 0 {
        data.insert(KEY_UPGRADES.into(), Tag::Int(upgrades as i32));
    }
    drop.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(data)));
}

/// A placed barrel that held something gets it back on its first tick.
pub(crate) fn place_block(host: &mut dyn Host, event: &mut PlaceBlockEvent<'_>) {
    if &*event.stack.item != BARREL {
        return;
    }
    if let Some(held) = carried(event.stack) {
        placed().lock().unwrap().insert((host.dimension(), event.pos), held);
    }
}

/// The item it holds, how many stacks and how full, or "Empty"; and its upgrades.
pub(crate) fn tooltip(host: &dyn Host, event: &mut TooltipEvent<'_>) {
    if &*event.stack.item != BARREL {
        return;
    }
    let (item, count, upgrades) = carried(event.stack).unwrap_or_default();
    if item.is_empty() {
        event.lines.push((Text::literal("Empty"), 0xFFAA_AAAA));
    } else {
        let name: Vec<String> = item
            .rsplit(':')
            .next()
            .unwrap_or(&item)
            .split('_')
            .map(|word| {
                let mut letters = word.chars();
                letters.next().map_or(String::new(), |c| c.to_uppercase().chain(letters).collect())
            })
            .collect();
        let stack_size = host.max_stack_size(&item).unwrap_or(64).max(1);
        let stacks = count as f32 / stack_size as f32;
        let room = (BASE_STACKS << upgrades.min(MAX_UPGRADES)) as f32;
        let percent = (stacks * 100.0 / room).ceil().clamp(1.0, 100.0) as u32;
        event.lines.push((Text::literal(name.join(" ")), 0xFFAA_AAAA));
        event.lines.push((Text::literal(format!("{count} items, {stacks:.1} stacks")), 0xFFAA_AAAA));
        event.lines.push((Text::literal(format!("{percent}% full")), 0xFFAA_AAAA));
    }
    if upgrades > 0 {
        let s = if upgrades == 1 { "" } else { "s" };
        event.lines.push((Text::literal(format!("{upgrades} Storage Upgrade{s}")), 0xFFAA_AAAA));
    }
}

/// What a barrel item holds, from its custom data: its item and count (empty and 0 for none),
/// and its Storage Upgrades.
fn carried(stack: &ItemStack) -> Option<(String, u32, u32)> {
    let Tag::Compound(data) = stack.data()?.other("minecraft:custom_data")? else {
        return None;
    };
    let (item, count) = match (get(data, KEY_ITEM), get(data, KEY_COUNT)) {
        (Some(Tag::String(s)), Some(Tag::Int(n))) if *n > 0 => (s.to_string_lossy(), *n as u32),
        _ => (String::new(), 0),
    };
    let upgrades = match get(data, KEY_UPGRADES) {
        Some(Tag::Int(n)) if *n > 0 => *n as u32,
        _ => 0,
    };
    (count > 0 || upgrades > 0).then_some((item, count, upgrades))
}

/// `map[name]`, with the key type taken from the map (the game's NBT string isn't exported).
fn get<'m, K: Ord + for<'a> From<&'a str>, V>(map: &'m std::collections::BTreeMap<K, V>, name: &str) -> Option<&'m V> {
    map.get(&K::from(name))
}

/// The `&'static` name `Host::dimension` gives for a level, so both maps key alike.
fn dimension_key(dimension: &str) -> &'static str {
    match dimension {
        "minecraft:overworld" => "minecraft:overworld",
        "minecraft:the_nether" => "minecraft:the_nether",
        "minecraft:the_end" => "minecraft:the_end",
        other => Box::leak(other.to_owned().into_boxed_str()),
    }
}

pub(crate) fn draw(host: &dyn Host, event: &mut ScreenDrawEvent<'_>) {
    let [x, y] = event.origin;
    let get = |i: usize| event.data.get(i).copied().unwrap_or(0).max(0) as u32;
    let text = |batch: &mut HudBatch, s: &str, at: [f32; 2]| host.text(batch, s, at, 0x40_4040, false);
    text(event.batch, "Storage Barrel", [x + 8.0, y + 6.0]);
    text(event.batch, "Inventory", [x + 8.0, y + 73.0]);
    // An arrow from the input to the output.
    host.fill(event.batch, [x + 66.0, y + 39.0, 40.0, 8.0], 0xFF8B_8B8B);
    let total = get(D_COUNT_LOW) | (get(D_COUNT_HIGH) << 15);
    let stacks = BASE_STACKS << get(D_UPGRADES).min(MAX_UPGRADES);
    let capacity = stacks * get(D_STACK_SIZE).max(1);
    text(event.batch, &format!("{total} / {capacity}"), [x + 8.0, y + 20.0]);
    // A fill bar under the slots.
    let (bx, by, bw) = (x + 44.0, y + 56.0, 90.0);
    host.fill(event.batch, [bx, by, bw, 4.0], 0xFF37_3737);
    let f = bw * (total as f32 / capacity as f32).min(1.0);
    host.fill(event.batch, [bx, by, f, 4.0], 0xFF5C_B85C);
    text(event.batch, &format!("{stacks} stacks"), [x + 44.0, y + 64.0]);
}
