//! The Compacting Drawer: one material kept as a count of its smallest form, offered in each of its
//! forms at once (nuggets, ingots, blocks). What goes in any form is added to the count; each output
//! slot shows as much of its form as the count covers, so taking from one takes from them all. It
//! holds 32 stacks of the largest form, doubled by each Storage Upgrade, to 32768 with ten. Broken,
//! it drops as a drawer that still holds its count and upgrades, as a Storage Barrel does.
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use horizon_client::mod_api::{
    BlockEntityTickEvent, Compound, Direction, Flow, Host, HudBatch, ItemStack, LootDropsEvent,
    MenuSlotRulesEvent, ModContainer, PlaceBlockEvent, ScreenDrawEvent, Tag, Text, TooltipEvent,
    UseItemOnEvent,
};

pub(crate) const DRAWER: &str = "horizon_chests:compacting_drawer";
const UPGRADE: &str = "horizon_chests:storage_upgrade";

/// Each material's forms, smallest first, with how many of the smallest each is.
const MATERIALS: [&[(&str, u32)]; 22] = [
    &[("minecraft:iron_nugget", 1), ("minecraft:iron_ingot", 9), ("minecraft:iron_block", 81)],
    &[("minecraft:gold_nugget", 1), ("minecraft:gold_ingot", 9), ("minecraft:gold_block", 81)],
    &[("minecraft:copper_nugget", 1), ("minecraft:copper_ingot", 9), ("minecraft:copper_block", 81)],
    &[("minecraft:netherite_ingot", 1), ("minecraft:netherite_block", 9)],
    &[("minecraft:diamond", 1), ("minecraft:diamond_block", 9)],
    &[("minecraft:emerald", 1), ("minecraft:emerald_block", 9)],
    &[("minecraft:lapis_lazuli", 1), ("minecraft:lapis_block", 9)],
    &[("minecraft:redstone", 1), ("minecraft:redstone_block", 9)],
    &[("minecraft:coal", 1), ("minecraft:coal_block", 9)],
    &[("minecraft:raw_iron", 1), ("minecraft:raw_iron_block", 9)],
    &[("minecraft:raw_gold", 1), ("minecraft:raw_gold_block", 9)],
    &[("minecraft:raw_copper", 1), ("minecraft:raw_copper_block", 9)],
    &[("minecraft:wheat", 1), ("minecraft:hay_block", 9)],
    &[("minecraft:slime_ball", 1), ("minecraft:slime_block", 9)],
    &[("minecraft:bone_meal", 1), ("minecraft:bone_block", 9)],
    &[("minecraft:dried_kelp", 1), ("minecraft:dried_kelp_block", 9)],
    &[("minecraft:quartz", 1), ("minecraft:quartz_block", 4)],
    &[("minecraft:glowstone_dust", 1), ("minecraft:glowstone", 4)],
    &[("minecraft:amethyst_shard", 1), ("minecraft:amethyst_block", 4)],
    &[("minecraft:clay_ball", 1), ("minecraft:clay", 4)],
    &[("minecraft:snowball", 1), ("minecraft:snow_block", 4)],
    &[("minecraft:honeycomb", 1), ("minecraft:honeycomb_block", 4)],
];

const INPUT: usize = 0;
const OUTPUTS: std::ops::Range<usize> = 1..4;
const UPGRADES: usize = 4;
const SLOTS: usize = 5;
const BASE_STACKS: u64 = 32;
const MAX_UPGRADES: u32 = 10;

// Data slots (16-bit on the wire): the count in two halves, the upgrades, and the material plus one.
const D_LOW: usize = 0;
const D_HIGH: usize = 1;
const D_UPGRADES: usize = 2;
const D_MATERIAL: usize = 3;
const DATA_SLOTS: usize = 4;

const KEY_ITEM: &str = "drawer_item";
const KEY_TOTAL: &str = "drawer_total";
const KEY_UPGRADES: &str = "drawer_upgrades";

/// A drawer's material, count and upgrades.
type Held = (Option<usize>, u32, u32);
type Key = (&'static str, [i32; 3]);

/// Each drawer as of its last tick, for its drop, as the loot hook can't see its block entity.
fn contents() -> std::sync::MutexGuard<'static, HashMap<Key, Held>> {
    static MAP: OnceLock<Mutex<HashMap<Key, Held>>> = OnceLock::new();
    MAP.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
}

/// What a drawer placed this tick held as an item, for its first tick to take in.
fn placed() -> std::sync::MutexGuard<'static, HashMap<Key, Held>> {
    static MAP: OnceLock<Mutex<HashMap<Key, Held>>> = OnceLock::new();
    MAP.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
}

fn material_of(item: &str) -> Option<usize> {
    MATERIALS.iter().position(|forms| forms.iter().any(|(form, _)| *form == item))
}

fn plain(stack: &ItemStack) -> bool {
    stack.damage == 0 && stack.data().is_none_or(|d| d.is_empty())
}

/// What the drawer holds at most, in the material's smallest form.
fn capacity(material: usize, upgrades: u32) -> u32 {
    let largest = MATERIALS[material].last().map_or(1, |(_, ratio)| *ratio) as u64;
    ((BASE_STACKS << upgrades.min(MAX_UPGRADES)) * 64 * largest).min(i32::MAX as u64) as u32
}

pub(crate) fn container() -> ModContainer {
    ModContainer {
        slots: SLOTS,
        faces: std::array::from_fn(|_| vec![INPUT, 1, 2, 3]),
        inputs: vec![INPUT],
        outputs: OUTPUTS.collect(),
        menu: Some(DRAWER.to_owned()),
    }
}

pub(crate) fn screen_slots() -> Vec<Option<[f32; 2]>> {
    vec![
        Some([26.0, 35.0]),
        Some([80.0, 35.0]),
        Some([98.0, 35.0]),
        Some([116.0, 35.0]),
        Some([152.0, 52.0]),
    ]
}

fn int(nbt: &Compound, key: &str) -> i32 {
    match get(nbt, key) {
        Some(Tag::Int(n)) => *n,
        _ => 0,
    }
}

pub(crate) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
    let pos = event.pos;
    let key = (host.dimension(), pos);
    let be = &mut **event.entity;
    if be.items.len() < SLOTS {
        be.items.resize(SLOTS, None);
    }
    let mut material = usize::try_from(int(&be.nbt, "material") - 1).ok().filter(|m| *m < MATERIALS.len());
    let mut total = int(&be.nbt, "total").max(0) as u32;
    if let Some((m, t, u)) = placed().remove(&key) {
        (material, total) = (m, t);
        if u > 0 && be.items[UPGRADES].is_none() {
            be.items[UPGRADES] = Some(ItemStack::new(UPGRADE, u.min(MAX_UPGRADES)));
        }
    }
    let upgrades = be.items[UPGRADES]
        .as_ref()
        .filter(|s| &*s.item == UPGRADE)
        .map_or(0, |s| s.count.min(MAX_UPGRADES));

    // What was taken from the outputs since the last tick comes off the count.
    if let Some(m) = material {
        let shown: Vec<i32> = match get(&be.nbt, "shown") {
            Some(Tag::IntArray(v)) => v.clone(),
            _ => Vec::new(),
        };
        for (i, (_, ratio)) in MATERIALS[m].iter().enumerate() {
            let now = be.items[OUTPUTS.start + i].as_ref().map_or(0, |s| s.count);
            let was = shown.get(i).copied().unwrap_or(0).max(0) as u32;
            total = total.saturating_sub(was.saturating_sub(now) * ratio);
        }
    }
    if total == 0 {
        material = None;
    }

    // Take in what the input holds, when it is this drawer's material and there is room.
    if let Some(input) = &mut be.items[INPUT]
        && plain(input)
        && let Some(m) = material_of(&input.item).filter(|m| material.is_none_or(|held| held == *m))
    {
        let ratio = MATERIALS[m].iter().find(|(form, _)| *form == &*input.item).map_or(1, |(_, r)| *r);
        let room = capacity(m, upgrades).saturating_sub(total) / ratio;
        let took = room.min(input.count);
        if took > 0 {
            material = Some(m);
            total += took * ratio;
            input.count -= took;
        }
        if input.count == 0 {
            be.items[INPUT] = None;
        }
    }

    // Each output shows as much of its form as the count covers.
    let mut shown = vec![0; OUTPUTS.len()];
    for (i, slot) in OUTPUTS.enumerate() {
        let form = material.and_then(|m| MATERIALS[m].get(i));
        be.items[slot] = form.and_then(|(item, ratio)| {
            let most = host.max_stack_size(item).unwrap_or(64);
            let count = most.min(total / ratio);
            shown[i] = count as i32;
            (count > 0).then(|| ItemStack::new(item, count))
        });
    }
    be.nbt.insert("shown".into(), Tag::IntArray(shown));
    be.nbt.insert("material".into(), Tag::Int(material.map_or(0, |m| m as i32 + 1)));
    be.nbt.insert("total".into(), Tag::Int(total as i32));
    be.data.resize(DATA_SLOTS, 0);
    be.data[D_LOW] = (total & 0x7FFF) as i32;
    be.data[D_HIGH] = (total >> 15) as i32;
    be.data[D_UPGRADES] = upgrades as i32;
    be.data[D_MATERIAL] = material.map_or(0, |m| m as i32 + 1);
    contents().insert(key, (material, total, upgrades));
}

/// The input takes what it can compact, the upgrade slot Storage Upgrades; the outputs only give.
pub(crate) fn slot_rule(event: &mut MenuSlotRulesEvent<'_>) {
    match event.slot {
        INPUT => event.may_place = plain(event.stack) && material_of(&event.stack.item).is_some(),
        UPGRADES => {
            event.may_place = &*event.stack.item == UPGRADE;
            event.max_stack = MAX_UPGRADES;
        }
        _ => event.may_place = false,
    }
}

/// A sneaking right click with an item it can compact puts the held stack in.
pub(crate) fn use_item_on(host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
    if host.block(event.pos) != DRAWER || !plain(event.stack) || material_of(&event.stack.item).is_none() {
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

/// A broken drawer's drop carries what it held.
pub(crate) fn loot_drops(event: &mut LootDropsEvent<'_>) {
    let Some((pos, _)) = event.context.block else {
        return;
    };
    let Some(drop) = event.drops.iter_mut().find(|s| &*s.item == DRAWER) else {
        return;
    };
    let dimension = match event.context.dimension {
        "minecraft:the_nether" => "minecraft:the_nether",
        "minecraft:the_end" => "minecraft:the_end",
        _ => "minecraft:overworld",
    };
    let Some((material, total, upgrades)) = contents().remove(&(dimension, pos)) else {
        return;
    };
    if total == 0 && upgrades == 0 {
        return;
    }
    let mut data = Compound::new();
    if let Some(m) = material.filter(|_| total > 0) {
        data.insert(KEY_ITEM.into(), Tag::String(MATERIALS[m][0].0.into()));
        data.insert(KEY_TOTAL.into(), Tag::Int(total as i32));
    }
    if upgrades > 0 {
        data.insert(KEY_UPGRADES.into(), Tag::Int(upgrades as i32));
    }
    drop.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(data)));
}

/// A placed drawer that held something gets it back on its first tick.
pub(crate) fn place_block(host: &mut dyn Host, event: &mut PlaceBlockEvent<'_>) {
    if &*event.stack.item != DRAWER {
        return;
    }
    if let Some(held) = carried(event.stack) {
        placed().insert((host.dimension(), event.pos), held);
    }
}

fn carried(stack: &ItemStack) -> Option<Held> {
    let Tag::Compound(data) = stack.data()?.other("minecraft:custom_data")? else {
        return None;
    };
    let material = match get(data, KEY_ITEM) {
        Some(Tag::String(s)) => material_of(&s.to_string_lossy()),
        _ => None,
    };
    let total = int(data, KEY_TOTAL).max(0) as u32;
    let upgrades = int(data, KEY_UPGRADES).max(0) as u32;
    (total > 0 || upgrades > 0).then_some((material, total, upgrades))
}

/// A name from an item's id: `minecraft:iron_ingot`, Iron Ingot.
fn title(item: &str) -> String {
    let words: Vec<String> = item
        .rsplit(':')
        .next()
        .unwrap_or(item)
        .split('_')
        .map(|word| {
            let mut letters = word.chars();
            letters.next().map_or(String::new(), |c| c.to_uppercase().chain(letters).collect())
        })
        .collect();
    words.join(" ")
}

pub(crate) fn tooltip(event: &mut TooltipEvent<'_>) {
    if &*event.stack.item != DRAWER {
        return;
    }
    let (material, total, upgrades) = carried(event.stack).unwrap_or_default();
    match material.filter(|_| total > 0) {
        Some(m) => {
            let percent = (total as f32 * 100.0 / capacity(m, upgrades) as f32).ceil().clamp(1.0, 100.0) as u32;
            event.lines.push((Text::literal(format!("{total} {}", title(MATERIALS[m][0].0))), 0xFFAA_AAAA));
            event.lines.push((Text::literal(format!("{percent}% full")), 0xFFAA_AAAA));
        }
        None => event.lines.push((Text::literal("Empty"), 0xFFAA_AAAA)),
    }
    if upgrades > 0 {
        let s = if upgrades == 1 { "" } else { "s" };
        event.lines.push((Text::literal(format!("{upgrades} Storage Upgrade{s}")), 0xFFAA_AAAA));
    }
}

pub(crate) fn draw(host: &dyn Host, event: &mut ScreenDrawEvent<'_>) {
    let [x, y] = event.origin;
    let get_data = |i: usize| event.data.get(i).copied().unwrap_or(0).max(0) as u32;
    let text = |batch: &mut HudBatch, s: &str, at: [f32; 2]| host.text(batch, s, at, 0x40_4040, false);
    text(event.batch, "Compacting Drawer", [x + 8.0, y + 6.0]);
    text(event.batch, "Inventory", [x + 8.0, y + 73.0]);
    host.fill(event.batch, [x + 48.0, y + 39.0, 28.0, 8.0], 0xFF8B_8B8B);
    let total = get_data(D_LOW) | (get_data(D_HIGH) << 15);
    let material = (get_data(D_MATERIAL) as usize).checked_sub(1).filter(|m| *m < MATERIALS.len());
    let Some(m) = material else {
        text(event.batch, "Empty", [x + 8.0, y + 20.0]);
        return;
    };
    let room = capacity(m, get_data(D_UPGRADES));
    text(event.batch, &format!("{total} / {room} {}", title(MATERIALS[m][0].0)), [x + 8.0, y + 20.0]);
    let (bx, by, bw) = (x + 26.0, y + 60.0, 108.0);
    host.fill(event.batch, [bx, by, bw, 4.0], 0xFF37_3737);
    host.fill(event.batch, [bx, by, bw * (total as f32 / room.max(1) as f32).min(1.0), 4.0], 0xFF5C_B85C);
}

/// `map[name]`, with the key type taken from the map (the game's NBT string isn't exported).
fn get<'m, K: Ord + for<'a> From<&'a str>, V>(map: &'m std::collections::BTreeMap<K, V>, name: &str) -> Option<&'m V> {
    map.get(&K::from(name))
}
