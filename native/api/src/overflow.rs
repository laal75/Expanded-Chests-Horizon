//! Chests side by side share the load: every 8 ticks a full chest (every slot taken, as a machine or
//! tube filling it leaves it) hands the stack in its last slot to an Expanded Chest touching it, on
//! any side, so whatever fills it carries on into the next. A chest never hands back to the one that
//! just filled it, so two full chests don't pass a stack to and fro; that ends once it has room again.
use horizon_client::mod_api::{BlockEntityTickEvent, Direction, Host};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const EVERY: u64 = 8;

/// The chest each chest last took overflow from, by level and position.
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

pub(crate) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>, slots: usize, is_chest: impl Fn(&str) -> bool) {
    let pos = event.pos;
    let key = (host.dimension().to_owned(), pos);
    // Each chest counts its own ticks, started apart by position so a wall of them doesn't all look
    // at once.
    {
        let mut counts = counts().lock().unwrap_or_else(|e| e.into_inner());
        let count = counts.entry(key.clone()).or_insert_with(|| (pos[0] ^ pos[1] ^ pos[2]).rem_euclid(EVERY as i32) as u8);
        *count = (*count + 1) % EVERY as u8;
        if *count != 0 {
            return;
        }
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
