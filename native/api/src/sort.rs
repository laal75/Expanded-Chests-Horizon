//! A sort button on each chest's screen, at the right end of its title row, as Horizon's own
//! chest screens have: pressed, the chest's stacks of one kind merge into as few full stacks as
//! fit and the kinds line up in id order. The press goes to the server as a menu button.
use horizon_client::mod_api::{Host, ItemStack};

/// The menu button id a sort press sends.
pub(crate) const SORT_BUTTON: i32 = 0;
pub(crate) const SIZE: f32 = 10.0;

/// Whether `at`, from the screen's top left, is on the button whose top left is `button`.
pub(crate) fn over(button: [f32; 2], at: [f32; 2]) -> bool {
    (button[0]..button[0] + SIZE).contains(&at[0]) && (button[1]..button[1] + SIZE).contains(&at[1])
}

/// A bordered square with three bars, as Horizon draws its own.
pub(crate) fn draw(host: &dyn Host, batch: &mut horizon_client::mod_api::HudBatch, at: [f32; 2]) {
    let [x, y] = at;
    host.fill(batch, [x, y, SIZE, SIZE], 0xFF37_3737);
    host.fill(batch, [x + 1.0, y + 1.0, SIZE - 2.0, SIZE - 2.0], 0xFFA0_A0A0);
    for (i, width) in [6.0, 4.0, 2.0].into_iter().enumerate() {
        host.fill(batch, [x + 2.0, y + 2.0 + 2.0 * i as f32, width, 1.0], 0xFF37_3737);
    }
}

fn same_item(a: &ItemStack, b: &ItemStack) -> bool {
    a.item == b.item && a.damage == b.damage && a.extra == b.extra
}

/// Merges and orders the stacks in the first `slots` slots.
pub(crate) fn sort(host: &dyn Host, items: &mut Vec<Option<ItemStack>>, slots: usize) {
    if items.len() < slots {
        items.resize(slots, None);
    }
    let mut held: Vec<ItemStack> = items[..slots].iter_mut().filter_map(Option::take).collect();
    held.sort_by(|a, b| a.item.cmp(&b.item).then(b.count.cmp(&a.count)));
    let mut merged: Vec<ItemStack> = Vec::new();
    for stack in held {
        let most = host.max_stack_size(&stack.item).unwrap_or(64).max(1);
        let mut left = stack;
        if let Some(last) = merged.last_mut().filter(|l| same_item(l, &left)) {
            let moved = most.saturating_sub(last.count).min(left.count);
            last.count += moved;
            left.count -= moved;
        }
        while left.count > 0 {
            let mut part = left.clone();
            part.count = left.count.min(most);
            left.count -= part.count;
            merged.push(part);
        }
    }
    let mut merged = merged.into_iter();
    for slot in items[..slots].iter_mut() {
        *slot = merged.next();
    }
}
