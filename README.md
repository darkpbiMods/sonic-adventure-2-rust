# Sonic Adventure 2 Rust

A reusable **Sonic character controller and CPU animation sampler in Rust**, developed against Sonic Adventure 2 Battle for GameCube. This is a character project, not a port of the complete game or an official SEGA release.

The playable character was accepted in a Windows/Xbox controller playtest. The development demo contains a skate bowl, an enclosed round tunnel for full loops, targets and ring rows. This repository contains the code and local import tools; the extracted game data and private playable distribution are not uploaded.

## Features

- Momentum, running, braking, jump, slope alignment and collision adapters.
- Shared Action button: somersault chains, held spin dash and rolling uncurl.
- Bounce, homing attack, charged Light Speed Attack and ring Light Dash.
- Normal and high-speed somersault animations; recovered Flame Ring particles.
- Original model/motion decoding and CPU pose sampling, separate from rendering.
- A fixed 60 Hz simulation with an independent host-world example.

The controller combines translated original routines with host integration code. It does not execute the original PowerPC program at runtime. Camera, some world interactions, rendering/audio output and parts of action dispatch are adapted. See [limitations](docs/RELEASE-NOTES.md).

## Get started

Requirements: Rust 1.99.0, Python 3.11 or newer, and your own supported game data. Windows/MSVC is the tested environment; install the C++ build tools and Windows SDK when using the MSVC Rust toolchain. Other operating systems have not been playtested. No emulator is required to run the Rust code.

Supported data: **Sonic Adventure 2 Battle USA, GameCube, GSNE8P revision 0**. The importer verifies the exact executable hash and rejects other revisions. It accepts an uncompressed ISO/GCM, CISO, an extracted `main.dol`, or a directory containing `main.dol`. Decompress a `.7z` archive yourself first; RVZ and other regions are not supported by this importer.

```sh
python tools/import_assets.py --game /path/to/your/game.iso
cargo run --manifest-path playable/Cargo.toml --locked --no-default-features --example headless
cargo run --manifest-path playable/Cargo.toml --locked --no-default-features --example headless -- --source-terrain
cargo test --manifest-path playable/Cargo.toml --locked --no-default-features
```

Generated data goes into ignored local folders. The importer does not download games. Import is required before compiling the `playable` crate because its numeric data is embedded at build time. The `runtime` library itself can be checked without game data:

```sh
cargo check --manifest-path runtime/Cargo.toml --locked --lib
```

## Put the character in another Rust project

Keep the `playable` and `runtime` folders together, then add a path dependency:

```toml
[dependencies]
sonic = { package = "sonic-playground", path = "../sonic-adventure-2-rust/playable", default-features = false }
```

Implement `sonic::simulation::World` for your engine's collision, targets, rings, events and spawn. Construct `Player::in_world(&world)` and call `tick_in(input, &world)` at 60 Hz, or use `Clock::advance_in` to retain input edges across variable frame rates. Read position, orientation and animation state to drive your renderer. The host example supplies an entirely separate world and validates movement, attacks and reset.

See [the complete headless example](playable/examples/headless.rs) and [the integration contract](docs/INTEGRATION.md). No Bevy, Godot, Zelda or other game-specific plug-in is provided yet. Full-sphere source collision is demonstrated on the authored tunnel; arbitrary host terrain still needs testing.

## Import original poses

Pass a disc image or a directory with `main.dol`, `plcommtn.prs`, `sonicmtn.prs` and `sonicmdl.prs`:

```sh
python tools/import_assets.py --game /path/to/your/game.iso --animations
cargo run --manifest-path playable/Cargo.toml --locked --no-default-features --example pose
```

This creates an ignored CPU model/motion bundle and demonstrates sampling it. It does not create the private demo's full texture/audio presentation package. Hosts must supply their own rendering/material integration and local assets. The optional desktop renderer source is retained for reference; enabling its `renderer` feature alone is not an asset installer.

## Controls in the private demo

Xbox left stick moves; A jumps/homing attacks. B/X are aliases of one contextual Action button: short press/release somersaults, hold charges, release launches, airborne press bounces, rolling press uncurls, and a press near forward rings Light Dashes. Full charged readiness stays held through jumping until released. Menu contains bowl, tunnel and ring practice starts.

## Verification and scope

The private development workspace has 282 passing headless tests across 50 suites after the final character changes, plus original-executable comparison fixtures and rendered checks. Those proprietary-data fixtures are not part of this repository. The tests included here cover the portable controls/moves, ring movement, fire-particle lifecycle and authored slope arena after local data import. CI checks only the asset-free library/import tooling and upload contents; it does not claim to test game-data-dependent gameplay without the data.

The latest small high-speed somersault addition has automated checks; its controller feel has not separately been confirmed by the user. Exact whole-dispatcher, special collision, GPU and audio-mixer equivalence remains outside this accepted character milestone.

## License and game data

Project code: [MIT](LICENSE). The license does not grant rights to Sonic Adventure 2, its names, game executable, art, animations or audio. No ROM, extracted game data, emulator, private logs or built executable is distributed here. See [provenance](docs/PROVENANCE.md).
