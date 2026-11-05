# Changelog

Versions follow [semver](https://semver.org): `tools/version.py <version>` sets one everywhere,
and each release is tagged `v<version>`. New entries go under **Unreleased**.

## Unreleased

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
