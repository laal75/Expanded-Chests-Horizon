//! Expanded Chests: four tiers of chest bigger than vanilla's, and a storage barrel that holds
//! up to 32768 stacks of one item (barrel.rs). The blocks, menus and recipes come from the content
//! mod; this crate gives them block entities, slots and screens. Hoppers, shift-clicks and
//! breaking (which spills the items) work as they do for any container.
use horizon_client::mod_api::{
    BlockEntityContainerEvent, BlockEntityEvent, BlockEntityTickEvent, BlockEntityTicksEvent,
    BlockEntityUpdateTagEvent, Compound, DestroyBlockEvent, Flow, Hooks, Host, LootDropsEvent, MenuButtonEvent,
    MenuSlotRulesEvent, ModContainer, NativeScreen, PlaceBlockEvent, ScreenClickEvent, ScreenDrawEvent, ScreenEvent,
    ScreenTooltipEvent, TooltipEvent, UseItemOnEvent,
};

mod barrel;
mod drawer;
mod keep;
mod overflow;
mod sort;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(HorizonChests)
}

/// For other mods (an optional dependency on `horizon_chests_core`): the item the storage barrel
/// at `pos` in level `dimension` (`minecraft:overworld`) holds, as of its last tick; None when it
/// is empty or no barrel is there. Horizon Machines' tubes send each item to the barrel that
/// holds it.
pub fn barrel_item(dimension: &str, pos: [i32; 3]) -> Option<String> {
    barrel::stored_item(dimension, pos)
}

/// A chest tier: its slots, as a grid of columns by rows.
struct Tier {
    block: &'static str,
    title: &'static str,
    cols: usize,
    rows: usize,
}

// The content generator (tools/gen_content.py) has the same grids. Six rows at most, as a double
// chest has, so every screen is 222 high and fits a small window; bigger tiers grow wider.
const TIERS: [Tier; 4] = [
    Tier { block: "horizon_chests:iron_chest", title: "Iron Chest", cols: 9, rows: 6 },
    Tier { block: "horizon_chests:gold_chest", title: "Gold Chest", cols: 12, rows: 6 },
    Tier { block: "horizon_chests:diamond_chest", title: "Diamond Chest", cols: 15, rows: 6 },
    Tier { block: "horizon_chests:obsidian_chest", title: "Obsidian Chest", cols: 18, rows: 6 },
];

fn tier(block: &str) -> Option<&'static Tier> {
    TIERS.iter().find(|t| t.block == block)
}

const SLOT: f32 = 18.0;
/// The grid's first row, under the title.
const GRID_Y: f32 = 18.0;
/// Between the grid and the inventory, as vanilla's chest has.
const GAP: f32 = 14.0;

impl Tier {
    fn slots(&self) -> usize {
        self.cols * self.rows
    }

    /// Vanilla's 176, or wider for more than 9 columns.
    fn width(&self) -> f32 {
        (self.cols as f32 * SLOT + 14.0).max(176.0)
    }

    fn grid_x(&self) -> f32 {
        (self.width() - self.cols as f32 * SLOT) / 2.0 + 1.0
    }

    fn inventory_x(&self) -> f32 {
        (self.width() - 9.0 * SLOT) / 2.0 + 1.0
    }

    fn inventory_y(&self) -> f32 {
        GRID_Y + self.rows as f32 * SLOT + GAP
    }

    /// The sort button: at the right end of the title row, over the grid's last column.
    fn sort_at(&self) -> [f32; 2] {
        [self.grid_x() + self.cols as f32 * SLOT - 2.0 - sort::SIZE, 5.0]
    }
}

struct HorizonChests;

impl Hooks for HorizonChests {
    fn block_entity(
        &self,
        _host: &mut dyn Host,
        event: &mut BlockEntityEvent<'_>,
    ) -> Option<(String, Compound)> {
        (tier(event.block).is_some() || event.block == barrel::BARREL || event.block == drawer::DRAWER)
            .then(|| (event.block.to_owned(), Compound::new()))
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        // The barrel moves its count in and out of its slots; a full chest hands on to the next
        // (overflow.rs).
        if event.block == barrel::BARREL || event.block == drawer::DRAWER || tier(event.block).is_some() {
            event.ticks = true;
        }
    }

    fn block_entity_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
        if event.entity.id == barrel::BARREL {
            barrel::tick(host, event);
            overflow::barrel_tick(host, event);
        } else if event.entity.id == drawer::DRAWER {
            drawer::tick(host, event);
        } else if let Some(t) = tier(&event.entity.id) {
            overflow::tick(host, event, t.slots(), |b| tier(b).is_some());
        }
    }

    fn block_entity_container(
        &self,
        _host: &dyn Host,
        event: &mut BlockEntityContainerEvent<'_>,
    ) -> Option<ModContainer> {
        if event.block == barrel::BARREL {
            return Some(barrel::container());
        }
        if event.block == drawer::DRAWER {
            return Some(drawer::container());
        }
        // Every side reaches every slot, both ways, as a chest's.
        let t = tier(event.block)?;
        let all: Vec<usize> = (0..t.slots()).collect();
        Some(ModContainer {
            slots: t.slots(),
            faces: std::array::from_fn(|_| all.clone()),
            inputs: all.clone(),
            outputs: all,
            menu: Some(t.block.to_owned()),
        })
    }

    fn block_entity_update_tag(&self, event: &mut BlockEntityUpdateTagEvent<'_>) -> Option<Compound> {
        // Clients only draw the block, so they need nothing of the entity.
        (tier(&event.entity.id).is_some() || event.entity.id == barrel::BARREL || event.entity.id == drawer::DRAWER)
            .then(Compound::new)
    }

    fn screen(&self, _host: &dyn Host, event: &mut ScreenEvent<'_>) -> Option<NativeScreen> {
        if event.block == drawer::DRAWER {
            return Some(NativeScreen {
                width: 176.0,
                height: 166.0,
                slots: drawer::screen_slots(),
                inventory_y: 84.0,
                inventory_x: 8.0,
                own_labels: true,
                ..NativeScreen::default()
            });
        }
        if event.block == barrel::BARREL {
            return Some(NativeScreen {
                width: 176.0,
                height: 166.0,
                slots: barrel::screen_slots(),
                inventory_y: 84.0,
                inventory_x: 8.0,
                own_labels: true,
                ..NativeScreen::default()
            });
        }
        let t = tier(event.block)?;
        let (gx, inventory_y) = (t.grid_x(), t.inventory_y());
        let slots = (0..t.slots())
            .map(|i| Some([gx + (i % t.cols) as f32 * SLOT, GRID_Y + (i / t.cols) as f32 * SLOT]))
            .collect();
        Some(NativeScreen {
            width: t.width(),
            // The inventory's three rows, a gap and the hotbar, as vanilla's.
            height: inventory_y + 82.0,
            slots,
            inventory_y,
            inventory_x: t.inventory_x(),
            // screen_draw writes the title and "Inventory" where the wider screens need them.
            own_labels: true,
            ..NativeScreen::default()
        })
    }

    fn screen_draw(&self, host: &dyn Host, event: &mut ScreenDrawEvent<'_>) {
        if event.block == barrel::BARREL {
            barrel::draw(host, event);
            return;
        }
        if event.block == drawer::DRAWER {
            drawer::draw(host, event);
            return;
        }
        let Some(t) = tier(event.block) else {
            return;
        };
        let [x, y] = event.origin;
        host.text(event.batch, t.title, [x + t.grid_x(), y + 6.0], 0x40_4040, false);
        host.text(
            event.batch,
            "Inventory",
            [x + t.inventory_x(), y + t.inventory_y() - 11.0],
            0x40_4040,
            false,
        );
        let [bx, by] = t.sort_at();
        sort::draw(host, event.batch, [x + bx, y + by]);
    }

    fn screen_click(&self, _host: &dyn Host, event: &mut ScreenClickEvent<'_>) -> Option<i32> {
        let t = tier(event.block)?;
        (!event.right && sort::over(t.sort_at(), event.at)).then_some(sort::SORT_BUTTON)
    }

    fn screen_tooltip(&self, _host: &dyn Host, event: &mut ScreenTooltipEvent<'_>) {
        if tier(event.block).is_some_and(|t| sort::over(t.sort_at(), event.at)) {
            event.lines.push("Sort".to_owned());
        }
    }

    fn menu_button(&self, host: &mut dyn Host, event: &mut MenuButtonEvent<'_>) {
        if event.id != sort::SORT_BUTTON {
            return;
        }
        let Some(t) = tier(&event.entity.id) else {
            return;
        };
        let slots = t.slots();
        sort::sort(host, &mut event.entity.items, slots);
    }

    fn menu_slot_rules(&self, event: &mut MenuSlotRulesEvent<'_>) {
        if event.menu == barrel::BARREL {
            barrel::slot_rule(event);
        } else if event.menu == drawer::DRAWER {
            drawer::slot_rule(event);
        }
    }

    fn use_item_on(&self, host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
        if drawer::use_item_on(host, event) == Flow::Cancel {
            return Flow::Cancel;
        }
        barrel::use_item_on(host, event)
    }

    /// A player breaking a chest: its items go with the chest (keep.rs).
    fn destroy_block(&self, host: &mut dyn Host, event: &mut DestroyBlockEvent) -> Flow {
        let block = host.block(event.pos);
        if let Some(t) = tier(&block) {
            keep::destroy_block(host, event.pos, t.slots());
        } else if block == barrel::BARREL {
            // Its count rides on the drop already; this keeps the input and output slots too.
            keep::destroy_block(host, event.pos, 2);
        } else if block == drawer::DRAWER {
            // Its count rides on the drop already; only the input has real items to keep.
            keep::destroy_block(host, event.pos, 1);
        }
        Flow::Continue
    }

    fn loot_drops(&self, _host: &mut dyn Host, event: &mut LootDropsEvent<'_>) {
        barrel::loot_drops(event);
        drawer::loot_drops(event);
        keep::loot_drops(event, |item| tier(item).is_some() || item == barrel::BARREL || item == drawer::DRAWER);
    }

    fn place_block(&self, host: &mut dyn Host, event: &mut PlaceBlockEvent<'_>) {
        barrel::place_block(host, event);
        drawer::place_block(host, event);
        if tier(&event.stack.item).is_some() || &*event.stack.item == barrel::BARREL || &*event.stack.item == drawer::DRAWER {
            keep::place_block(host, event);
        }
    }

    fn tooltip(&self, host: &dyn Host, event: &mut TooltipEvent<'_>) {
        barrel::tooltip(host, event);
        drawer::tooltip(event);
        if let Some(t) = tier(&event.stack.item) {
            keep::tooltip(host, event, t.slots());
        }
    }
}
