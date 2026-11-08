//! Chests side by side share the load: every 8 ticks a full chest (every slot taken, as a machine or
//! tube filling it leaves it) hands on, so whatever fills it carries on into the next. First, to a
//! Storage Barrel touching it that holds an item the chest has: a stack of that item. Then the stack
//! in its last slot to an Expanded Chest touching it, on any side.
//!
//! A full Storage Barrel does the same with what waits in its input: it goes to a Storage Barrel
//! touching it that holds the same item. An empty barrel takes nothing this way, and passes nothing
//! on. Nothing hands back to the one that just filled it, so two full ones don't pass a stack to and
//! fro; that ends once it has room again.
use horizon_client::mod_api::{BlockEntityTickEvent, Direction, Host};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::barrel;

const EVERY: u64 = 8;

/// The chest or barrel each one last took overflow from, by level and position.
fn came_from() -> &'static Mutex<HashMap<(String, [i32; 3]), [i32; 3]>> {
    static MAP: OnceLock<Mutex<HashMap<(String, [i32; 3]), [i32; 3]>>> = OnceLock::new();
    MAP.get_or_init(Default::default)
}

fn counts() -> &'static Mutex<HashMap<(String, [i32; 3]), u8>> {
    static MAP: OnceLock<Mutex<HashMap<(String, [i32; 3]), u8>>> = OnceLock::new();
    MAP.get_or_init(Default::default)
}

fn step(pos: [i32; 3], face: Direction) -> [i32; 3] {
    let n = face.normal();
    [pos[0] + n[0], pos[1] + n[1], pos[2] + n[2]]
}

/// Each one counts its own ticks, started apart by position so a wall of them doesn't all look at
/// once: whether this is its tick to hand on.
fn due(key: &(String, [i32; 3])) -> bool {
    let pos = key.1;
    let mut counts = counts().lock().unwrap_or_else(|e| e.into_inner());
    let count = counts.entry(key.clone()).or_insert_with(|| (pos[0] ^ pos[1] ^ pos[2]).rem_euclid(EVERY as i32) as u8);
    *count = (*count + 1) % EVERY as u8;
    *count == 0
}

pub(crate) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>, slots: usize, is_chest: impl Fn(&str) -> bool) {
    let pos = event.pos;
    let key = (host.dimension().to_owned(), pos);
    if !due(&key) {
        return;
    }
    let items = &mut event.entity.items;
    if items.len() < slots {
        items.resize(slots, None);
    }
    let mut map = came_from().lock().unwrap_or_else(|e| e.into_inner());
    if items[..slots].iter().any(Option::is_none) {
        map.remove(&key);
        return;
    }
    let back = map.get(&key).copied();
    // A storage barrel that holds something the chest has takes a stack of it.
    for face in Direction::ALL {
        let next = step(pos, face);
        if Some(next) == back || host.block(next) != barrel::BARREL {
            continue;
        }
        let Some(held) = barrel::stored_item(&key.0, next) else {
            continue;
        };
        let Some(slot) = (0..slots)
            .rev()
            .find(|&s| items[s].as_ref().is_some_and(|st| *st.item == *held && barrel::plain(st)))
        else {
            continue;
        };
        let Some(stack) = items[slot].take() else {
            continue;
        };
        let before = stack.count;
        let left = host.insert_item(next, face.opposite(), stack).filter(|s| s.count > 0);
        let moved = before - left.as_ref().map_or(0, |s| s.count);
        items[slot] = left;
        if moved > 0 {
            map.insert((key.0.clone(), next), pos);
        }
    }
    // Then the last slot's stack, to an Expanded Chest.
    let last = slots - 1;
    for face in Direction::ALL {
        let next = step(pos, face);
        if Some(next) == back || !is_chest(&host.block(next)) {
            continue;
        }
        let Some(stack) = items[last].take() else {
            return;
        };
        let before = stack.count;
        let left = host.insert_item(next, face.opposite(), stack).filter(|s| s.count > 0);
        let moved = before - left.as_ref().map_or(0, |s| s.count);
        items[last] = left;
        if moved > 0 {
            map.insert((key.0.clone(), next), pos);
        }
        if items[last].is_none() {
            return;
        }
    }
}

/// A full Storage Barrel, after its own tick: what still waits in its input, when it is the
/// barrel's own item, goes to a storage barrel touching it that holds the same.
pub(crate) fn barrel_tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
    let pos = event.pos;
    let key = (host.dimension().to_owned(), pos);
    if !due(&key) {
        return;
    }
    let mut map = came_from().lock().unwrap_or_else(|e| e.into_inner());
    let items = &mut event.entity.items;
    let waiting = items.get(barrel::INPUT).cloned().flatten().filter(|s| s.count > 0);
    let (Some(waiting), Some(own)) = (waiting, barrel::stored_item(&key.0, pos)) else {
        map.remove(&key);
        return;
    };
    if *waiting.item != *own {
        return;
    }
    let back = map.get(&key).copied();
    for face in Direction::ALL {
        let next = step(pos, face);
        if Some(next) == back || host.block(next) != barrel::BARREL {
            continue;
        }
        if barrel::stored_item(&key.0, next).as_deref() != Some(own.as_str()) {
            continue;
        }
        let Some(stack) = items[barrel::INPUT].take() else {
            return;
        };
        let before = stack.count;
        let left = host.insert_item(next, face.opposite(), stack).filter(|s| s.count > 0);
        let moved = before - left.as_ref().map_or(0, |s| s.count);
        items[barrel::INPUT] = left;
        if moved > 0 {
            map.insert((key.0.clone(), next), pos);
        }
        if items[barrel::INPUT].is_none() {
            return;
        }
    }
}
