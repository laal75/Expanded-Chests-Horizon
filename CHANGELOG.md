# Changelog

Versions follow [semver](https://semver.org): `tools/version.py <version>` sets one everywhere,
and each release is tagged `v<version>`. New entries go under **Unreleased**.

## Unreleased

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
