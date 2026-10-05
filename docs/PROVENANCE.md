# Provenance and boundaries

This project was developed by translating and adapting selected routines from a locally supplied GameCube Sonic Adventure 2 Battle executable, with tests comparing selected records and outputs against original execution. It is not a recovery of SEGA's original source tree, a complete game recompilation, or evidence of bit-identical gameplay for every situation.

The supported main.dol SHA-256 is `a53335308d1f820be236fd2b6342fee850eaf048eb26aac21ddb1903b060db2e`. Source addresses in Rust comments identify the bounded translated routines. Field interpretation research also used the X-Hax SA2 structure naming noted in the relevant modules. Third-party Rust dependencies are fetched by Cargo under their own licenses; they are not vendored here.

The code archive intentionally excludes executable data sections, model/motion bundles, audio, textures, ROMs, emulator binaries and original-execution fixture captures. The importer derives needed local data from the user's supplied supported image. The optional `reference-tests` feature refers to development-only fixtures that are not distributed and is not part of the public quick-start test command.

The public importer reproduces the numeric controller data and CPU pose bundle. The full private desktop presentation asset pipeline is not a turnkey public download. An engine integrating the controller must supply its own World adapter and material/audio integration. This distinction is part of the project scope, not a claim that every Rust engine can load the character without integration work.
