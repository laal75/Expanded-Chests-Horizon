# Horizon Chests

Bigger chests and storage barrels for [Horizon](https://github.com/horizon-rust/horizon), the Rust
reimplementation of Minecraft: Java Edition. Everything in it is original: names, numbers, code and
textures.

## What it adds

### Chests

| Chest | Slots | Crafted from |
|---|---|---|
| Iron Chest | 54 (9 × 6) | A chest ringed with 8 iron ingots |
| Gold Chest | 72 (12 × 6) | An Iron Chest ringed with 8 gold ingots |
| Diamond Chest | 90 (15 × 6) | A Gold Chest with 4 diamonds and 4 glass |
| Netherite Chest | 108 (18 × 6) | A Diamond Chest with a netherite ingot and 7 obsidian; blast-proof |

Every chest is six rows tall, like a double chest, so its screen fits a small window; bigger tiers
grow wider. Hoppers reach every slot from any side, shift-clicks move stacks as a chest's do, and
breaking a chest spills what's in it.

### Storage Barrel

Holds one kind of item: 32 stacks, doubled by each **Storage Upgrade**, up to 1,024 stacks with
five. Hoppers and tubes fill and empty it like a chest. Break it and it drops as a barrel that still
holds its items; place it again to get them back.

| Item | Crafted from |
|---|---|
| Storage Barrel | A barrel, 4 iron ingots and 4 planks |
| Storage Upgrade | A chest, 4 iron ingots and 4 gold ingots |

Every recipe unlocks in the recipe book the first time you hold one of its ingredients.

## Works with other mods

- The chests are in the common `c:chests` block tag, and the storage barrel in `c:barrels`, so mods
  that read those tags treat them as chests and barrels.
- [Horizon Machines](https://github.com/laal75/horizon-machines)' item tubes connect to them, and
  fill each storage barrel with the item it already holds before using any chest.
- For other native mods: `horizon_chests_core::barrel_item(dimension, pos)` returns the item a
  storage barrel holds. Declare an optional dependency on `horizon_chests_core` to call it.

## Install

Horizon Chests is a Horizon **native mod**: the Horizon launcher compiles it into the game for you.
It has two halves, and you need both:

- the content (`content/`, or `horizon_chests.hmod`): blocks, items, recipes and textures
- the code (`native/`, or `horizon_chests_core.hnmod`): the chests' and barrel's slots and screens

You need the [Horizon launcher](https://github.com/horizon-rust/horizon). The launcher downloads its
own Rust toolchain the first time it builds a native mod, which takes 20 to 30 minutes; later builds
take a few minutes.

1. Download or clone this repository.
2. Build the code into an instance (a new one is safest while you try it):

   ```sh
   horizon-launcher instance create "Horizon Chests"
   horizon-launcher native-build native --instance <instance id> --trust-native-code
   ```

   `horizon-launcher instance list` shows the instance id. `--trust-native-code` confirms that you
   trust the code: native mods run with your user's rights, so read it first if you like.
3. Copy the `content/` folder into the instance's game folder as `horizon-mods/horizon_chests/`
   (or put `horizon_chests-<version>.hmod` from `python tools/pack.py` there).
4. Turn both halves on in the instance's `horizon-mods.txt`:

   ```text
   mod.horizon_chests:true
   mod.horizon_chests_core:true
   ```

5. Play the instance: `horizon-launcher play --instance <instance id>`.

To build it together with other native mods such as Horizon Machines, put their `native` folders (or
links to them) side by side in one folder and run `native-build` on that folder.

### Horizon version

The code builds on Horizon commit `67dad0d` (the 2026-11-04 Nightly, hooks API 359), set in
`native/horizon-mixin.toml` and `native/api/Cargo.toml`, the
same as Horizon Machines, so the two build into one game. The launcher fetches that version when it
builds.

## Development

- `content/` is **generated**: edit `tools/gen_content.py`, then run
  `python tools/gen_content.py content`. The script writes the textures too.
- `native/api/src/` is the code: `lib.rs` (the chests: block entities, slots and screens),
  `barrel.rs` (the storage barrel) and `keep.rs` (keeping contents when broken). The chest grids
  are in both the code and the generator.
- `python tools/pack.py` writes `dist/horizon_chests-<version>.hmod` (the content, for an
  instance's `horizon-mods/`) and `dist/horizon_chests_core-<version>.hnmod` (the code, for
  `horizon-native-mods/` or a GitHub release).

### Versions and releases

`VERSION` holds the version (semver); [CHANGELOG.md](CHANGELOG.md) says what each one changed.
Note changes under its **Unreleased** heading as you go. To release:

1. `python tools/version.py 0.3.0`: sets the version in `VERSION`, both manifests and
   `Cargo.toml`, regenerates the content, and dates the Unreleased notes as 0.3.0.
2. Commit, then `git tag v0.3.0` and push with `git push --follow-tags`.
3. `python tools/pack.py`, and attach the two files in `dist/` to a GitHub release for the tag.

`python tools/version.py` alone checks that every file agrees; `pack.py` refuses when they don't.

## License

MIT; see [LICENSE](LICENSE).
