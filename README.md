# Horizon Chests

Bigger chests for [Horizon](https://github.com/horizon-rust/horizon), in four tiers. Everything is
original: names, numbers, code and textures.

## Contents

| Chest | Slots | Crafted from |
|---|---|---|
| Iron Chest | 54 (9×6) | A chest ringed with 8 iron ingots |
| Gold Chest | 72 (12×6) | An Iron Chest ringed with 8 gold ingots |
| Diamond Chest | 90 (15×6) | A Gold Chest with 4 diamonds and 4 glass |
| Netherite Chest | 108 (18×6) | A Diamond Chest with a netherite ingot and 7 obsidian; blast-proof |

Every chest is six rows tall, as a double chest is, so its screen fits a small window; bigger tiers
grow wider. Hoppers reach every slot from any side, shift-clicks move stacks as a chest's do, and
breaking a chest spills what is in it.

## Layout

- `content/`: the content mod `horizon_chests`: blocks, menus, recipes, textures. **Generated**:
  edit `tools/gen_content.py`, then `python tools/gen_content.py content`.
- `native/`: the native trait mod `horizon_chests_core` (`api/src/lib.rs`): block entities, slots
  and screens. The grids are in both files.

## Build and play

The native half pins Horizon `02c8607` (hooks API 359), the same as Horizon Machines, so the two
build into one game.

```sh
horizon-launcher native-build native --instance <instance> --trust-native-code
```

To build it with other native mods, put their folders (or links to them) side by side in one
folder and build that. Copy `content/` to the instance's `horizon-mods/horizon_chests/`, and turn
on both `mod.horizon_chests` and `mod.horizon_chests_core` in its `horizon-mods.txt`.

## License

MIT; see [LICENSE](LICENSE).
