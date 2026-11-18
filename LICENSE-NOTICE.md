# Licence provenance

- Mod: `framedblocks` (FramedBlocks) by XFactHD — <https://github.com/XFactHD/FramedBlocks>
- Licence: **LGPL-3.0-only**, read from the campaign's jar audit for
  `FramedBlocks-10.6.3.jar` (`ports/_shared/source-map/atm10-sources.csv` row
  `framedblocks`), and from the upstream repository's `LICENSE`. Rank 1 (copyleft):
  publishable under the same licence with its complete corresponding source, and
  **local-only** until the maintainer approves publishing (ARCHITECTURE.md §8).
- Upstream version analysed: 11.4.0, branch `26.1`, commit
  `4f74728ea3d4f051b1fc96626e22bb04a7f4f8cc` (Minecraft Java 26.1.2). The ATM10 pack
  ships 10.6.1 for 1.21.1.
- This port is a **clean-room Rust rewrite** written from the mod's observable
  behaviour, its data files and its public documentation; no Java source was copied
  or translated.
- Carried content: `assets/framedblocks/blockstates/`, `assets/framedblocks/items/`,
  `assets/framedblocks/models/`, `assets/framedblocks/textures/block/framed_block.png`,
  `assets/framedblocks/textures/item/framed_hammer.png`,
  `assets/framedblocks/lang/en_us.json`, `data/framedblocks/**` and
  `data/minecraft/tags/block/mineable/axe.json`. These files come from the upstream
  repository under the same LGPL-3.0-only licence and are redistributed here under it.
- The complete corresponding source of this package is this directory: `src/`,
  `horizon-mod.toml`, `Cargo.toml`, `assets/`, `data/` and the `LICENSE` text beside
  this notice.
