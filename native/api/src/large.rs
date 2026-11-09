//! The Large Chest and the Large Barrel: hollow boxes built in the world. Each is a box of 3 to 15
//! blocks of air inside on every side, walled with planks or logs, with the same metal block on all
//! eight corners (iron x1, gold x2, diamond x4, obsidian x8), and up to one port on each of its six
//! sides (not on an edge). A Large Chest holds 27 slots for every block of space inside, times the
//! metal; a Large Barrel holds one item, 64 stacks for every block inside, times the metal.
//!
//! What a box holds is kept by its lowest inside corner in the world's
//! `horizon_chests/large_storage.json`, so it survives the box being opened up and built again.
//! Only plain items go in (as a storage barrel's: no names, damage or enchantments).
//!
//! A port is the box's hands: its 9 input slots take items in (hoppers, tubes, the storage
//! network), and its view slots hold a stack of each item kind, drawn out of the store and topped
//! up every tick, which hoppers, tubes and players take from. A Large Chest port has a view slot for
//! up to 247 kinds (a menu holds 256 slots at most), 27 to a page on its screen; a Large Barrel port has 9. The storage network
//! (Machine Tech) counts the rest from the port's block entity: `large_items`, by `box_x/y/z`.
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use horizon_client::mod_api::{
    BlockEntityTickEvent, Compound, Direction, Host, HudBatch, ItemStack, MenuButtonEvent, MenuSlotRulesEvent,
    ModContainer, ScreenClickEvent, ScreenDrawEvent, Tag,
};

pub(crate) const CHEST_PORT: &str = "horizon_chests:large_chest";
pub(crate) const BARREL_PORT: &str = "horizon_chests:large_barrel";

pub(crate) fn is_port(block: &str) -> bool {
    block == CHEST_PORT || block == BARREL_PORT
}

/// Slots 0 to 8 take in; the rest are the view slots.
const INPUTS: usize = 9;
const CHEST_VIEWS: usize = 247;
const BARREL_VIEWS: usize = 9;
/// A page of a Large Chest port's screen.
const PAGE: usize = 27;

fn views_of(block: &str) -> usize {
    if block == BARREL_PORT { BARREL_VIEWS } else { CHEST_VIEWS }
}

pub(crate) fn slots_of(block: &str) -> usize {
    INPUTS + views_of(block)
}

/// The corner metals: block, what it multiplies the room by, and its name.
const METALS: [(&str, u64, &str); 4] = [
    ("minecraft:iron_block", 1, "Iron"),
    ("minecraft:gold_block", 2, "Gold"),
    ("minecraft:diamond_block", 4, "Diamond"),
    ("minecraft:obsidian", 8, "Obsidian"),
];
const SLOTS_PER_BLOCK: u64 = 27;
const STACKS_PER_BLOCK: u64 = 64;
const LEAST_SIDE: i32 = 3;
const MOST_SIDE: i32 = 15;
/// How often a port looks at its box again, and how often the store is written.
const RECHECK: Duration = Duration::from_secs(1);
const SAVE_EVERY: Duration = Duration::from_secs(2);

// What is wrong with a box, worst last: the screen says it, and where.
const F_NO_SPACE: i32 = 1;
const F_TOO_BIG: i32 = 2;
const F_TOO_SMALL: i32 = 3;
const F_CORNER: i32 = 4;
const F_CORNERS_DIFFER: i32 = 5;
const F_EDGE: i32 = 6;
const F_SIDE: i32 = 7;
const F_TWO_PORTS: i32 = 8;
const F_OTHER_PORT: i32 = 9;
const F_INSIDE: i32 = 10;

type Fault = (i32, [i32; 3]);

fn is_air(block: &str) -> bool {
    block == "minecraft:air" || block == "minecraft:cave_air"
}

/// Planks and logs (and wood, stems and hyphae, stripped or not) for the edges and sides.
fn is_wood(block: &str) -> bool {
    block.ends_with("_planks")
        || block.ends_with("_log")
        || block.ends_with("_wood")
        || block.ends_with("_stem")
        || block.ends_with("_hyphae")
        || block == "minecraft:bamboo_block"
        || block == "minecraft:stripped_bamboo_block"
}

/// Only plain items go in: the store keeps a count, not a stack's name, damage or enchantments.
pub(crate) fn plain(stack: &ItemStack) -> bool {
    stack.damage == 0 && stack.data().is_none_or(|d| d.is_empty())
}

fn step(pos: [i32; 3], face: Direction) -> [i32; 3] {
    let n = face.normal();
    [pos[0] + n[0], pos[1] + n[1], pos[2] + n[2]]
}

#[derive(Clone, Debug)]
struct Shape {
    /// The space inside, lowest and highest corner.
    low: [i32; 3],
    high: [i32; 3],
    metal: usize,
    ports: Vec<[i32; 3]>,
}

impl Shape {
    fn size(&self) -> [i32; 3] {
        std::array::from_fn(|k| self.high[k] - self.low[k] + 1)
    }

    fn volume(&self) -> u64 {
        self.size().iter().map(|&n| n as u64).product()
    }

    /// A chest's room in slots, a barrel's in stacks.
    fn room(&self, barrel: bool) -> u64 {
        let per = if barrel { STACKS_PER_BLOCK } else { SLOTS_PER_BLOCK };
        per * self.volume() * METALS[self.metal].1
    }
}

/// The box the port at `pos` is a side of, or what is wrong (the worst fault any side found).
fn find(host: &dyn Host, pos: [i32; 3], block: &str) -> Result<Shape, Fault> {
    let mut worst: Fault = (F_NO_SPACE, pos);
    let note = |f: Fault, worst: &mut Fault| {
        if f.0 > worst.0 {
            *worst = f;
        }
    };
    for face in Direction::ALL {
        let start = step(pos, face);
        if !is_air(&host.block(start)) {
            continue;
        }
        let (mut low, mut high) = (start, start);
        let mut open = false;
        for k in 0..3 {
            let run = |way: i32| -> Option<i32> {
                for d in 1..=MOST_SIDE {
                    let mut at = start;
                    at[k] += way * d;
                    if !is_air(&host.block(at)) {
                        return Some(d - 1);
                    }
                }
                None
            };
            match (run(-1), run(1)) {
                (Some(a), Some(b)) => {
                    low[k] = start[k] - a;
                    high[k] = start[k] + b;
                }
                _ => {
                    open = true;
                    break;
                }
            }
        }
        if open || (0..3).any(|k| high[k] - low[k] + 1 > MOST_SIDE) {
            note((F_TOO_BIG, start), &mut worst);
            continue;
        }
        // The port has to be on one side of this space, not an edge.
        let within = (0..3).all(|k| (low[k] - 1..=high[k] + 1).contains(&pos[k]));
        let outside = (0..3).filter(|&k| pos[k] < low[k] || pos[k] > high[k]).count();
        if !within || outside != 1 {
            continue;
        }
        if (0..3).any(|k| high[k] - low[k] + 1 < LEAST_SIDE) {
            note((F_TOO_SMALL, start), &mut worst);
            continue;
        }
        match shell(host, low, high, block) {
            Ok(shape) => return Ok(shape),
            Err(f) => note(f, &mut worst),
        }
    }
    Err(worst)
}

/// Checks the walls around the space from `low` to `high`, and that the space is empty.
fn shell(host: &dyn Host, low: [i32; 3], high: [i32; 3], block: &str) -> Result<Shape, Fault> {
    let mut metal: Option<usize> = None;
    let mut ports = Vec::new();
    let mut sides = [false; 6];
    for x in low[0] - 1..=high[0] + 1 {
        for y in low[1] - 1..=high[1] + 1 {
            for z in low[2] - 1..=high[2] + 1 {
                let at = [x, y, z];
                let outside = (0..3).filter(|&k| at[k] < low[k] || at[k] > high[k]).count();
                if outside == 0 {
                    continue;
                }
                let b = host.block(at);
                match outside {
                    3 => {
                        let Some(m) = METALS.iter().position(|(id, ..)| *id == b) else {
                            return Err((F_CORNER, at));
                        };
                        match metal {
                            None => metal = Some(m),
                            Some(n) if n != m => return Err((F_CORNERS_DIFFER, at)),
                            _ => {}
                        }
                    }
                    2 => {
                        if !is_wood(&b) {
                            return Err((F_EDGE, at));
                        }
                    }
                    _ => {
                        if b == block {
                            let k = (0..3).find(|&k| at[k] < low[k] || at[k] > high[k]).unwrap_or(0);
                            let side = k * 2 + usize::from(at[k] > high[k]);
                            if sides[side] {
                                return Err((F_TWO_PORTS, at));
                            }
                            sides[side] = true;
                            ports.push(at);
                        } else if is_port(&b) {
                            return Err((F_OTHER_PORT, at));
                        } else if !is_wood(&b) {
                            return Err((F_SIDE, at));
                        }
                    }
                }
            }
        }
    }
    for x in low[0]..=high[0] {
        for y in low[1]..=high[1] {
            for z in low[2]..=high[2] {
                if !is_air(&host.block([x, y, z])) {
                    return Err((F_INSIDE, [x, y, z]));
                }
            }
        }
    }
    Ok(Shape { low, high, metal: metal.unwrap_or(0), ports })
}

#[derive(Default)]
struct Store {
    world: Option<PathBuf>,
    checked: Option<Instant>,
    saved: Option<Instant>,
    changed: bool,
    /// What each box holds past its ports' view slots, by dimension and lowest inside corner.
    boxes: HashMap<(String, [i32; 3]), BTreeMap<String, u64>>,
    /// What each port's view slots hold now, by dimension and position, with its box.
    shown: HashMap<(String, [i32; 3]), ([i32; 3], Vec<(String, u64)>)>,
    /// Each port's last look at its box.
    shapes: HashMap<(String, [i32; 3]), (Instant, Result<Shape, Fault>)>,
    /// The item each formed Large Barrel port's box holds ("" when empty), for `barrel_item`.
    barrels: HashMap<(String, [i32; 3]), String>,
}

fn store() -> MutexGuard<'static, Store> {
    static MAP: OnceLock<Mutex<Store>> = OnceLock::new();
    MAP.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
}

fn store_file(world: &std::path::Path) -> PathBuf {
    world.join("horizon_chests").join("large_storage.json")
}

/// The store, read again from the open world's file when the world changed.
fn loaded(host: &dyn Host) -> MutexGuard<'static, Store> {
    let mut s = store();
    if s.checked.is_some_and(|t| t.elapsed() < RECHECK) {
        return s;
    }
    s.checked = Some(Instant::now());
    let world = host.world_dir();
    if s.world != world {
        *s = Store { checked: s.checked, ..Store::default() };
        if let Some(text) = world.as_ref().and_then(|w| std::fs::read_to_string(store_file(w)).ok())
            && let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(&text)
        {
            for (key, value) in map {
                let parts: Vec<&str> = key.split(' ').collect();
                let ([dim, x, y, z], serde_json::Value::Object(items)) = (&parts[..], value) else {
                    continue;
                };
                let (Ok(x), Ok(y), Ok(z)) = (x.parse(), y.parse(), z.parse()) else {
                    continue;
                };
                let held: BTreeMap<String, u64> =
                    items.into_iter().filter_map(|(k, v)| v.as_u64().filter(|&n| n > 0).map(|n| (k, n))).collect();
                s.boxes.insert((dim.to_string(), [x, y, z]), held);
            }
        }
        s.world = world;
    }
    s
}

fn save(s: &mut Store) {
    if !s.changed || s.saved.is_some_and(|t| t.elapsed() < SAVE_EVERY) {
        return;
    }
    let Some(world) = s.world.clone() else {
        return;
    };
    let map: serde_json::Map<String, serde_json::Value> = s
        .boxes
        .iter()
        .filter(|(_, held)| held.values().any(|&n| n > 0))
        .map(|((dim, p), held)| {
            let items: serde_json::Map<String, serde_json::Value> =
                held.iter().filter(|(_, n)| **n > 0).map(|(k, n)| (k.clone(), serde_json::json!(n))).collect();
            (format!("{dim} {} {} {}", p[0], p[1], p[2]), serde_json::Value::Object(items))
        })
        .collect();
    let file = store_file(&world);
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(file, serde_json::Value::Object(map).to_string());
    s.saved = Some(Instant::now());
    s.changed = false;
}

/// The port's box, looked at again once a second.
fn shape_of(s: &mut Store, host: &dyn Host, dim: &str, pos: [i32; 3], block: &str) -> Result<Shape, Fault> {
    let key = (dim.to_owned(), pos);
    if let Some((at, shape)) = s.shapes.get(&key)
        && at.elapsed() < RECHECK
    {
        return shape.clone();
    }
    let shape = find(host, pos, block);
    s.shapes.insert(key, (Instant::now(), shape.clone()));
    shape
}

/// The item the Large Barrel port at `pos` holds; None for none there or an empty one.
pub(crate) fn barrel_item(dimension: &str, pos: [i32; 3]) -> Option<String> {
    store().barrels.get(&(dimension.to_owned(), pos)).filter(|i| !i.is_empty()).cloned()
}

fn stack_size(host: &dyn Host, item: &str) -> u64 {
    host.max_stack_size(item).unwrap_or(64).max(1) as u64
}

fn slots_for(count: u64, size: u64) -> u64 {
    count.div_ceil(size)
}

// Data slots (16-bit on the wire, so big numbers go in two 15-bit halves).
const D_STATE: usize = 0;
const D_METAL: usize = 1;
const D_SIZE: usize = 2;
const D_USED: usize = 5;
const D_ROOM: usize = 7;
const D_PAGE: usize = 9;
const D_PAGES: usize = 10;
const D_AT: usize = 11;
const D_KINDS: usize = 14;
pub(crate) const DATA_SLOTS: usize = 16;

fn split(data: &mut [i32], at: usize, n: u64) {
    let n = n.min((1 << 30) - 1);
    data[at] = (n & 0x7FFF) as i32;
    data[at + 1] = (n >> 15) as i32;
}

fn joined(data: &[i32], at: usize) -> u64 {
    let get = |i: usize| data.get(i).copied().unwrap_or(0).max(0) as u64;
    get(at) | (get(at + 1) << 15)
}

fn nbt_int<K: Ord + for<'a> From<&'a str>>(nbt: &BTreeMap<K, Tag>, key: &str) -> i32 {
    match nbt.get(&K::from(key)) {
        Some(Tag::Int(n)) => *n,
        _ => 0,
    }
}

fn nbt_remove<K: Ord + for<'a> From<&'a str>>(nbt: &mut BTreeMap<K, Tag>, key: &str) {
    nbt.remove(&K::from(key));
}

pub(crate) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
    let pos = event.pos;
    let dim = host.dimension();
    let block = event.entity.id.to_string();
    let barrel = block == BARREL_PORT;
    let views = views_of(&block);
    let be = &mut **event.entity;
    if be.items.len() < INPUTS + views {
        be.items.resize(INPUTS + views, None);
    }
    be.data.resize(DATA_SLOTS, 0);
    let mut s = loaded(&*host);
    let shape = shape_of(&mut s, &*host, dim, pos, &block);
    let me = (dim.to_owned(), pos);

    // What the view slots hold goes back to the store it came from (or, after a restart, this box's).
    let back_to = s.shown.remove(&me).map(|(b, _)| b).or(shape.as_ref().ok().map(|sh| sh.low));
    if let Some(b) = back_to {
        let mut moved = false;
        let store = s.boxes.entry((dim.to_owned(), b)).or_default();
        for slot in &mut be.items[INPUTS..INPUTS + views] {
            if let Some(stack) = slot.take() {
                *store.entry(stack.item.to_string()).or_default() += stack.count as u64;
                moved = true;
            }
        }
        s.changed |= moved;
    }

    let sh = match shape {
        Ok(sh) => sh,
        Err((code, at)) => {
            be.data[D_STATE] = code;
            for k in 0..3 {
                be.data[D_AT + k] = (at[k] - pos[k]).clamp(-30_000, 30_000);
            }
            for key in ["large_items", "box_x", "box_y", "box_z", "large_free"] {
                nbt_remove(&mut be.nbt, key);
            }
            be.nbt.insert("item".into(), Tag::String("".into()));
            s.barrels.remove(&me);
            save(&mut s);
            return;
        }
    };

    // Ports gone from the box no longer hold any of it.
    let low = sh.low;
    s.shown.retain(|(d, p), (b, _)| !(d == dim && *b == low && !sh.ports.contains(p)));
    let others: Vec<(String, u64)> = s
        .shown
        .iter()
        .filter(|((d, _), (b, _))| d == dim && *b == low)
        .flat_map(|(_, (_, held))| held.iter().cloned())
        .collect();
    let mut changed = false;
    let store = s.boxes.entry((dim.to_owned(), low)).or_default();
    // Everything the box holds: the store and the other ports' view slots.
    let mut total = store.clone();
    for (item, n) in &others {
        *total.entry(item.clone()).or_default() += n;
    }
    total.retain(|_, n| *n > 0);
    let room = sh.room(barrel);
    let mut used: u64 = if barrel { 0 } else { total.iter().map(|(i, n)| slots_for(*n, stack_size(&*host, i))).sum() };

    // Take in the inputs, as much as there is room for.
    for slot in 0..INPUTS {
        let Some(input) = be.items[slot].as_mut() else {
            continue;
        };
        if !plain(input) {
            continue;
        }
        let item = input.item.to_string();
        let size = stack_size(&*host, &item);
        let have = total.get(&item).copied().unwrap_or(0);
        let fits = if barrel {
            if total.keys().any(|k| *k != item) {
                continue;
            }
            (room * size).saturating_sub(have)
        } else {
            let partial = if have % size == 0 { 0 } else { size - have % size };
            partial + room.saturating_sub(used) * size
        };
        let took = fits.min(input.count as u64);
        if took > 0 {
            input.count -= took as u32;
            *store.entry(item.clone()).or_default() += took;
            *total.entry(item.clone()).or_default() += took;
            if !barrel {
                used = used - slots_for(have, size) + slots_for(have + took, size);
            }
            changed = true;
        }
        if input.count == 0 {
            be.items[slot] = None;
        }
    }

    // Fill the view slots: a stack of each kind, this page's kinds first.
    let kinds: Vec<String> = total.keys().cloned().collect();
    let pages = kinds.len().div_ceil(PAGE).max(1);
    let page = (nbt_int(&be.nbt, "page").max(0) as usize).min(pages - 1);
    be.nbt.insert("page".into(), Tag::Int(page as i32));
    let mut shown_now: Vec<(String, u64)> = Vec::new();
    let order: Vec<&String> = if barrel {
        kinds.first().map(|k| vec![k; views]).unwrap_or_default()
    } else {
        kinds.iter().cycle().skip(page * PAGE).take(kinds.len().min(views)).collect()
    };
    for (v, kind) in order.into_iter().enumerate() {
        let have = store.get(kind).copied().unwrap_or(0);
        let n = have.min(stack_size(&*host, kind));
        if n == 0 {
            continue;
        }
        *store.get_mut(kind).unwrap() -= n;
        be.items[INPUTS + v] = Some(ItemStack::new(kind, n as u32));
        shown_now.push((kind.clone(), n));
        changed = true;
    }
    store.retain(|_, n| *n > 0);

    // For the storage network: the rest of what the box holds, and where it is.
    let mut items = Compound::new();
    for (item, n) in store.iter() {
        items.insert(item.as_str().into(), Tag::Long(*n as i64));
    }
    be.nbt.insert("large_items".into(), Tag::Compound(items));
    for (k, key) in ["box_x", "box_y", "box_z"].iter().enumerate() {
        be.nbt.insert((*key).into(), Tag::Int(low[k]));
    }
    let held_item = kinds.first().cloned().unwrap_or_default();
    be.nbt.insert("item".into(), Tag::String(if barrel { held_item.as_str() } else { "" }.into()));
    be.nbt.insert("large_free".into(), Tag::Int(room.saturating_sub(used).min(i32::MAX as u64) as i32));

    // The screen's numbers.
    be.data[D_STATE] = 0;
    be.data[D_METAL] = sh.metal as i32;
    for (k, n) in sh.size().iter().enumerate() {
        be.data[D_SIZE + k] = *n;
    }
    if barrel {
        let size = kinds.first().map_or(64, |k| stack_size(&*host, k));
        split(&mut be.data, D_USED, total.values().sum());
        split(&mut be.data, D_ROOM, room * size);
    } else {
        split(&mut be.data, D_USED, used);
        split(&mut be.data, D_ROOM, room);
    }
    be.data[D_PAGE] = page as i32;
    be.data[D_PAGES] = pages as i32;
    be.data[D_KINDS] = kinds.len().min(30_000) as i32;

    s.shown.insert(me.clone(), (low, shown_now));
    if barrel {
        s.barrels.insert(me, held_item);
    }
    s.changed |= changed;
    save(&mut s);
}

/// A player breaking a port: what its view slots hold goes back to its box's store first, so it
/// doesn't spill. (Its inputs are real items and spill as a chest's do.)
pub(crate) fn destroy_block(host: &mut dyn Host, pos: [i32; 3], block: &str) {
    let dim = host.dimension();
    let mut s = loaded(&*host);
    let Some((low, _)) = s.shown.remove(&(dim.to_owned(), pos)) else {
        return;
    };
    s.barrels.remove(&(dim.to_owned(), pos));
    let mut back = Vec::new();
    for v in 0..views_of(block) {
        if let Some(stack) = host.extract_item(dim, pos, Direction::Up, INPUTS + v, u32::MAX) {
            back.push(stack);
        }
    }
    let store = s.boxes.entry((dim.to_owned(), low)).or_default();
    for stack in back {
        *store.entry(stack.item.to_string()).or_default() += stack.count as u64;
    }
    s.changed = true;
    save(&mut s);
}

pub(crate) fn container(block: &str) -> ModContainer {
    let views = views_of(block);
    let all: Vec<usize> = (0..INPUTS + views).collect();
    ModContainer {
        slots: INPUTS + views,
        faces: std::array::from_fn(|_| all.clone()),
        inputs: (0..INPUTS).collect(),
        outputs: (INPUTS..INPUTS + views).collect(),
        menu: Some(block.to_owned()),
    }
}

/// The inputs take plain items; nothing is put into a view slot.
pub(crate) fn slot_rule(event: &mut MenuSlotRulesEvent<'_>) {
    event.may_place = event.slot < INPUTS && plain(event.stack);
}

// The screens. A Large Chest port's: 27 view slots, the input under them, the page buttons.
const CHEST_INPUT_AT: [f32; 2] = [8.0, 80.0];
const PREV_AT: [f32; 4] = [124.0, 80.0, 12.0, 12.0];
const NEXT_AT: [f32; 4] = [158.0, 80.0, 12.0, 12.0];
const PREV: i32 = 1;
const NEXT: i32 = 2;
pub(crate) const CHEST_INVENTORY_Y: f32 = 112.0;
// A Large Barrel port's: the input, the count and a row of 9 view slots.
const BARREL_INPUT_AT: [f32; 2] = [8.0, 22.0];
pub(crate) const BARREL_INVENTORY_Y: f32 = 84.0;

pub(crate) fn screen_slots(block: &str) -> Vec<Option<[f32; 2]>> {
    let barrel = block == BARREL_PORT;
    let mut out: Vec<Option<[f32; 2]>> = vec![None; INPUTS + views_of(block)];
    out[0] = Some(if barrel { BARREL_INPUT_AT } else { CHEST_INPUT_AT });
    for v in 0..if barrel { BARREL_VIEWS } else { PAGE } {
        out[INPUTS + v] = Some(if barrel {
            [8.0 + 18.0 * v as f32, 50.0]
        } else {
            [8.0 + 18.0 * (v % 9) as f32, 18.0 + 18.0 * (v / 9) as f32]
        });
    }
    out
}

fn inside(at: [f32; 2], r: [f32; 4]) -> bool {
    (r[0]..r[0] + r[2]).contains(&at[0]) && (r[1]..r[1] + r[3]).contains(&at[1])
}

pub(crate) fn click(event: &ScreenClickEvent<'_>) -> Option<i32> {
    if event.block != CHEST_PORT || event.right {
        return None;
    }
    if inside(event.at, PREV_AT) {
        return Some(PREV);
    }
    inside(event.at, NEXT_AT).then_some(NEXT)
}

pub(crate) fn button(event: &mut MenuButtonEvent<'_>) {
    if event.entity.id != CHEST_PORT {
        return;
    }
    let page = nbt_int(&event.entity.nbt, "page");
    let next = match event.id {
        PREV => page - 1,
        NEXT => page + 1,
        _ => return,
    };
    // The tick keeps it within the pages there are.
    event.entity.nbt.insert("page".into(), Tag::Int(next.max(0)));
}

/// "3 east, 1 up" from the port, for where a fault is.
fn offset_words(d: [i32; 3]) -> String {
    let mut parts = Vec::new();
    for (n, plus, minus) in [(d[0], "east", "west"), (d[1], "up", "down"), (d[2], "south", "north")] {
        if n != 0 {
            parts.push(format!("{} {}", n.abs(), if n > 0 { plus } else { minus }));
        }
    }
    if parts.is_empty() { "here".to_owned() } else { parts.join(", ") }
}

fn fault_lines(code: i32) -> [&'static str; 2] {
    match code {
        F_TOO_BIG => ["No closed box: inside is", "more than 15 or open"],
        F_TOO_SMALL => ["Too small: 3 blocks", "of air inside at least"],
        F_CORNER => ["A corner isn't an iron, gold", "or diamond block or obsidian"],
        F_CORNERS_DIFFER => ["The corners aren't", "all the same metal"],
        F_EDGE => ["An edge isn't", "planks or logs"],
        F_SIDE => ["A side block isn't planks,", "logs or a port"],
        F_TWO_PORTS => ["Two ports on one side:", "one per side"],
        F_OTHER_PORT => ["The other kind of port", "is on this box"],
        F_INSIDE => ["Something is inside:", "it must be all air"],
        _ => ["Put it in a side of a", "hollow box of wood"],
    }
}

fn group(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn button_box(host: &dyn Host, batch: &mut HudBatch, origin: [f32; 2], r: [f32; 4], label: &str) {
    let (bx, by) = (origin[0] + r[0], origin[1] + r[1]);
    host.fill(batch, [bx, by, r[2], r[3]], 0xFF37_3737);
    host.fill(batch, [bx, by, r[2] - 1.0, r[3] - 1.0], 0xFFFF_FFFF);
    host.fill(batch, [bx + 1.0, by + 1.0, r[2] - 2.0, r[3] - 2.0], 0xFF8B_8B8B);
    host.text(batch, label, [bx + (r[2] - 5.0) / 2.0, by + 2.0], 0xFF_FFFF, true);
}

pub(crate) fn draw(host: &dyn Host, event: &mut ScreenDrawEvent<'_>) {
    let [x, y] = event.origin;
    let data: Vec<i32> = event.data.to_vec();
    let get = |i: usize| data.get(i).copied().unwrap_or(0);
    let text = |batch: &mut HudBatch, s: &str, at: [f32; 2]| host.text(batch, s, at, 0x40_4040, false);
    let barrel = event.block == BARREL_PORT;
    let title = if barrel { "Large Barrel" } else { "Large Chest" };
    text(event.batch, title, [x + 8.0, y + 6.0]);
    let inventory_y = if barrel { BARREL_INVENTORY_Y } else { CHEST_INVENTORY_Y };
    text(event.batch, "Inventory", [x + 8.0, y + inventory_y - 11.0]);
    let state = get(D_STATE);
    if state != 0 {
        // Not formed: what is wrong and where, over the slots.
        let (top, h) = if barrel { (18.0, 50.0) } else { (17.0, 56.0) };
        host.fill(event.batch, [x + 7.0, y + top, 162.0, h], 0xFFC6_C6C6);
        host.text(event.batch, "Not formed", [x + 8.0, y + top + 2.0], 0xA0_2020, false);
        let [a, b] = fault_lines(state);
        text(event.batch, a, [x + 8.0, y + top + 13.0]);
        text(event.batch, b, [x + 8.0, y + top + 23.0]);
        if state >= F_TOO_BIG {
            let at = offset_words([get(D_AT), get(D_AT + 1), get(D_AT + 2)]);
            text(event.batch, &format!("At: {at}"), [x + 8.0, y + top + 35.0]);
        }
        return;
    }
    let metal = METALS[get(D_METAL).clamp(0, 3) as usize].2;
    let size = format!("{metal} {}x{}x{}", get(D_SIZE), get(D_SIZE + 1), get(D_SIZE + 2));
    let w = size.chars().count() as f32 * 6.0;
    text(event.batch, &size, [x + 168.0 - w, y + 6.0]);
    let (used, room) = (joined(&data, D_USED), joined(&data, D_ROOM));
    let (line_at, bar_at) = if barrel { ([x + 30.0, y + 23.0], [x + 30.0, y + 34.0]) } else { ([x + 30.0, y + 81.0], [x + 30.0, y + 92.0]) };
    let unit = if barrel { "" } else { " slots" };
    text(event.batch, &format!("{} / {}{unit}", group(used), group(room)), line_at);
    let bw = if barrel { 138.0 } else { 88.0 };
    host.fill(event.batch, [bar_at[0], bar_at[1], bw, 4.0], 0xFF37_3737);
    let f = bw * (used as f32 / room.max(1) as f32).min(1.0);
    host.fill(event.batch, [bar_at[0], bar_at[1], f, 4.0], 0xFF5C_B85C);
    if !barrel {
        button_box(host, event.batch, [x, y], PREV_AT, "<");
        button_box(host, event.batch, [x, y], NEXT_AT, ">");
        let page = format!("{}/{}", get(D_PAGE) + 1, get(D_PAGES).max(1));
        let pw = page.chars().count() as f32 * 6.0;
        text(event.batch, &page, [x + 147.0 - pw / 2.0, y + 82.0]);
        let kinds = get(D_KINDS);
        if kinds > 0 {
            let s = if kinds == 1 { "" } else { "s" };
            text(event.batch, &format!("{kinds} kind{s}"), [x + 30.0, y + 99.0]);
        }
    }
}
