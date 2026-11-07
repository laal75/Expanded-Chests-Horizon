# Changelog

Versions follow [semver](https://semver.org): `tools/version.py <version>` sets one everywhere,
and each release is tagged `v<version>`. New entries go under **Unreleased**.

## Unreleased

## 0.5.0 - 2026-11-07

- A full chest hands the stack in its last slot to an Expanded Chest touching it, on any side, every 8 ticks, so a machine or tube filling it carries on into the next. It never hands straight back to the chest that just filled it.

## 0.4.1 - 2026-11-07

- The Storage Barrel's and Compacting Drawer's upgrade slots sit higher, clear of the inventory's sort button.

## 0.4.0 - 2026-11-06

- **Compacting Drawer**: one material offered in all its forms, so nuggets, ingots and blocks convert
  freely. Recipe: a Storage Barrel between two pistons.

## 0.3.3 - 2026-11-06

- Storage Barrels' item tooltip names the item, its stacks and how full it is, or says Empty.
- The chests' item tooltips say Empty, or how many percent full they are.

## 0.3.2 - 2026-11-06

- Storage Barrels take 10 Storage Upgrades, up from 5: up to 32,768 stacks.

## 0.3.1 - 2026-11-05

### Added
- **Chest Upgrades** (Iron, Gold, Diamond, Obsidian): sneak and right-click a chest with the next
  tier's upgrade to upgrade it in place, keeping its items. They replace upgrading with loose
  materials in your inventory.
- A Sort button on every chest's screen (right end of the title row): merges stacks and orders them by item.

### Removed
- The Netherite Chest (replaced by the Obsidian Chest in 0.3.0). Any still placed disappear, with
  what they hold: empty them first.

### Changed
- The mod is now called Expanded Chests (its ids are unchanged, so worlds keep their chests).

### Added
- Upgrade a chest where it stands, keeping what it holds: sneak and right-click it with the next
  tier's materials (8 iron ingots on a single chest, 8 gold ingots on an Iron Chest, 4 diamonds
  and 4 glass on a Gold Chest, 8 obsidian on a Diamond Chest).

### Fixed
- The barrel's and kept chests' tooltip lines were invisible (their colour had no alpha).

## 0.3.0 - 2026-11-05

### Changed
- The top chest is now the **Obsidian Chest** (108 slots, blast-proof), crafted from a Diamond
  Chest ringed with 8 obsidian. It replaces the Netherite Chest, which is no longer crafted or in
  the creative tab; Netherite Chests already placed keep working.

## 0.2.1 - 2026-11-05

### Added
- A broken storage barrel keeps its Storage Upgrades as well as its items, and placing it puts
  them back; its tooltip shows them.

### Changed
- Builds on Horizon `67dad0d` (the 2026-11-04 Nightly) instead of `02c8607`, which launcher
  0.2.38 no longer builds. No code changes were needed.
- `tools/pack.py` moves the packs already in `dist/` to `past-backups/<version>/<time made>/`
  before writing new ones.

## 0.2.0 - 2026-11-03

### Added
- Storage Barrel: holds one kind of item, 32 stacks to start; each Storage Upgrade doubles it, to
  1024 stacks with five. Hoppers and tubes fill and empty it; sneak and right-click with an item to
  put it in by hand.
- Storage Upgrade, and recipes for both.
- Chests and the Storage Barrel keep what they hold when a player breaks them: the dropped item
  carries it (its tooltip says how much) and gives it back when placed.
- The chests are in the common `c:chests` block tag and the barrel in `c:barrels`, so other mods'
  pipes (Horizon Machines' item tubes) connect to them.
- `barrel_item` for other mods to read what a barrel holds.

## 0.1.0 - 2026-11-03

### Added
- Iron, Gold, Diamond and Netherite chests: 54, 72, 90 and 108 slots, six rows tall, each crafted
  around the tier before it. The Netherite Chest survives explosions.
