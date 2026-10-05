# Sonic Rust host integration

The saved character demo is v0.44.1. The user accepted the v0.44.0 playtest for packaging; v0.44.1 adds the tested high-speed somersault sequence. This is a reusable Rust character controller with recovered SA2 assets and translated routines, not a port of the whole game. See [current release notes](RELEASE-NOTES.md) for controls, features and remaining fidelity differences. Earlier versioned notes below are historical.

B/X share one contextual Action input. Supply `action_held` continuously and `bounce_pressed` only on a fresh Action edge; `charge` is the lower-level direct-charge compatibility input. Ring dash has priority near eligible rings. Original fire particles are available through `Player::fire_spawn` and the renderer-independent `FireEffects` pool. A host advances effects once after each simulation tick.

For full-sphere source terrain, input heading is interpreted before contact pitch/roll. The desktop tube camera uses `controller::surface_camera_direction` to convert camera-plane input to that heading. Ordinary camera input continues using `camera_direction`. Inverted loops are verified on the authored static tube; arbitrary engine terrain still requires integration testing.

## Build and tick

Depend on `sonic-playground` with `default-features = false` for simulation, controller processing and CPU pose sampling without Macroquad. Keep the sibling `runtime` crate and embedded assets available when building. The default `renderer` feature adds the desktop renderer and audio adapters.

Implement `World`, construct `Player::in_world(&world)`, and use `tick_in(Input, &world)` once per 1/60 second. Alternatively, use `Clock::advance_in` to accumulate elapsed time and retain button edges. Use `reset_in` for your spawn and slot counts. `new`, `tick` and `reset` select demo defaults.

Coordinates are Y-up; one host unit equals ten SA2 units. Source velocity in units per tick converts to host units per second by multiplying by six. `Input::direction` is a world-space X/Z vector with magnitude at most one. The host owns camera-relative conversion. Keep `charge` (seconds toward readiness) separate from `spin_charge_speed` (stored source speed). Rendering must not advance simulation.

From the source root, run the independent consumer:

```powershell
cargo run --manifest-path playable/Cargo.toml --locked --no-default-features --example headless
```

Add `-- --source-terrain` to run the same consumer with a translated two-triangle source collision plane. Both modes are verified independently. The normal dependency tree with default features disabled contains no Macroquad or Miniquad renderer dependency.

It supplies its own world outside the demo arena, tests movement, jump, bounce, landing, callbacks and reset, then applies queued damage to two targets while an immune third survives. It demonstrates the API, not complete SA2 fidelity.

## Contact and terrain

`World::move_player` resolves one bounded movement substep. The host owns geometry queries, sweeps, walls, support classification and loss of support. For descending or persistent support, call `Player::resolve_support(resolved_feet, unit_normal)`. Preserve the incoming grounded flag until this call so it can identify a new landing. Do not accept an upward jump crossing as support.

The shared method validates finite contact data and a unit upward normal before mutation, resolves supported velocity, applies the demo landing/bounce response and resets air-dash availability. It does not detect contact. Set `grounded = false` when the host finds no support. Its mutable Player argument is still a broad integration boundary.

`ground_normal` must return the upward world-space normal at the resolved position. Decorators must forward it. Player normalizes valid normals and falls back to Y-up for invalid/zero values. Ground motion consumes this normal through recovered terrain alignment and forces. On new support, alignment sees the incoming contact status before it is marked established. Presentation reads the recovered model basis where available.

Hosts can return stable `StaticTerrain` geometry from `World::source_terrain`. Vertices use source units (ten per host unit), Y up, with outward winding. The current adapter accepts at most 1024 triangles. Geometry may vary between ticks, but triangle ordering and object identities must remain stable for a player's lifetime, and all queries within a tick must use the same snapshot. Use StaticTerrain::with_attachments to associate triangles with platform task addresses; triangles sharing a task are grouped into one collision object. Implement World::prepare_platform_memory and World::platform_callback for attached objects. The default callback rejects attached platform execution. PlatformCourse provides a translating-platform example; arbitrary rotating-platform behavior is not established. Ordinary source ground frames run the recovered full position routine once per tick and skip `move_player` substeps. The controller retains source records; reconciliation supplies locomotion velocity to the host. Ordinary jump/fall and bounce also use recovered source collision. Bounce retains its own action and extension records, consumes prior support before physics, and uses the alternate source terrain path selected by bounce entry flags. Targeted homing, charged attacks and targetless fallback now use recovered terrain collision too. Fallback preserves source state when a target disappears; supported fallback landing is decided on the following frame before ground physics. B cancellation clears source attack records and continues recovered falling physics. Other unsupported action paths can still call `move_player`, so hosts must implement both paths.

DemoWorld supplies its floor, ramp and block faces through this path. Two 600-second stationary incline tests report zero displacement; course checks cover ramp ascent, wall containment and platform-edge support loss. These checks do not establish complete game collision fidelity: authored static tube loops are tested, while arbitrary wall running, rotating platforms and arbitrary surface transitions remain unverified. Translating platforms have bounded horizontal/vertical carry, takeoff, bounce, attack-recovery and support-loss sequence coverage; this does not establish every moving-platform behavior. Six flat/slope airborne cases and six bounce physics cases match complete original movement records; these bounded comparisons do not establish all collision behavior. Worlds returning `None` retain the prior collision path. The separate `incline_visual` example still uses the older host projection path.

## Objects and events

Target and ring indices are stable identities for the lifetime of a Player. Return `None` when removing a slot; do not compact or reuse it until reset. Set slot counts before constructing/resetting Player. Moving target positions are queried each tick.

`target` supplies a sphere center, `target_radius` its finite nonnegative extent, and `target_attackable` eligibility. The recovered homing/charged-attack overlap uses Sonic's source radius and center height. Roll/bounce damage still has separate approximate contact behavior. `attack_path_clear` is a host visibility query, not a substitute for movement collision.

Version 0.25 adds `target_origin`, the entity position used for homing distance ranking; its default is `target`. Override it when the aim center is offset from the entity origin. `homing_target_flags` defaults to bit 0x40 for attackable targets, and `homing_target_kind` defaults to zero. Type 17 is excluded from the normal list; type 10 is excluded by the selector. Eligible entries retain host slot order, with at most 656 entries and a strict distance of less than 10 host units between entity origins. Aim direction uses the supplied world sphere center, while ranking uses squared source-unit distance between origins. Equal distances retain the first entry. These defaults are explicit host mappings; they do not recover another game's collision metadata automatically.

The v0.25 homing list retains ordered slots and distance values between ticks. `tick_in` refreshes it after `after_tick`; selection reads current aim centers against those cached distances. Call `refresh_homing_candidates` after a separate host collision phase or an out-of-band teleport if that changes the appropriate snapshot. Before the first completed tick, or following reset, acquisition bootstraps from the current world. This bootstrap and the host's collision ordering are explicit adaptations. Active pursuit reselection is included from v0.26 onward; see the detailed ordering and cancellation notes below.

Charged pursuit uses `light_target_category` (default category 2) and `light_target_distance` (default squared source-unit distance). The original target-list producer and full damage categories are not implemented. `homing_contact_response` defaults to the authored `DemoUpward`; the explicit source category options cover only their documented contact cases, including the recovered category0 orientation/impulse when source records are available. The legacy SourceCategory0Upright enum name remains for compatibility; host-only behavior retains its documented approximation.

Queue `ActionEvent` in `World::event`, then apply damage/audio after ticking. `TargetHit(index)` and `RingCollected(index)` identify the affected slot. Several simulation ticks may occur per render frame. Do not reenter or mutate Player from callbacks. `after_tick` provides one completed-tick snapshot. Explicit reset does not emit an automatic Respawn event.

## Animation and rendering

Read `source_animation_clock`, `model_rotation` and source lean for the recovered presentation path. The shared clock covers supported locomotion, jumps, homing, spin, readiness, bounce, fall and landing transitions. Different normal/bounce rigs switch directly rather than blending incompatible joints. Ordinary flat landing selection uses surviving air records; stationary clips finish through the clock, with movement/charge/support loss interrupting them.

Ordinary idle now selects original variants 88/89 naturally and slope poses 4/5 through the surface-aware selector. The host supplies demo-local frame/RNG state; this is not the original global random stream or contextual voice output. Clip 90 has pose/timing coverage, but its gameplay activation is not established.

`SourcePose::from_math_bundle(audit, assets, math_data)` samples original-space mesh vertices without a GPU. `sample(animation, frame, action, lean)` requires a finite in-range frame. Prepare same-model transitions once with `prepare_transition`, then sample progress in [0,1]. Use `draw_packets::assemble` with the corresponding draw plan before host coordinate conversion. Prepared transitions own their workspace and share immutable math.

The desktop `SourceRenderer` consumes the source-render fixture and demo assets. Its cache reuses unchanged local geometry while world transforms remain live. Keep executable and asset versions together. Runtime execution needs the packaged extracted data, not a full ROM or main.dol. The complete original-executable verification suite still needs external fixtures.

## Controller and audio

The desktop convention is Xbox: left stick move, A jump/airborne homing, airborne B bounce, X spin charge and release (full readiness launches Light Speed Attack), right stick camera, triggers zoom, Menu pause and View help. A host can construct Input directly without Windows polling. Physical Xbox feel remains a playtest requirement.

`--audio-preview` enables the desktop action/charge adapters. Charge pause/resume preserves the native playback cursor; reset and charge exit clear it. Host events, recovered pitch/envelope routines and extracted samples are available, but mixer/gain/resampling and audible quality are not verified as equivalent to SA2.

## Remaining integration limits

The v0.26 preview adds active homing reselection from the retained candidate list. Prior target contact is resolved before reselection; an empty or rejected list enters the recovered fallback, which does not reacquire targets. Active pursuit also consumes source terrain support on the following decision frame and runs the selected ground controller that same frame, as fallback already did. Invalid/disabled host volumes and blocked visibility retain explicit host cancellation policies. The moving-target reference comparison covers original pursuit and selection with free-space integration, not the complete original contact dispatcher. These changes are included in v0.26; the preserved v0.25 package predates them.

This is a reusable Rust prototype with recovered movement and presentation components, not a full original action dispatcher or complete SA2 engine. Remaining work includes collision/action context, special landing behavior, global idle context, lighting/reflections, dash-trail appearance, mixer fidelity, stable real-world performance and physical controller testing. Zelda integration is not included. Passing the scripted course and reference comparisons proves their covered cases only; the overall acceptance gates remain open.

The v0.26 source also hands active pursuit timeout to the source falling controller on the same tick when source collision records are available. Existing target overlap takes priority over timeout; timeout suppresses a simultaneous bounce request. This is covered by host overlap and terrain tests plus the scripted rendered course; complete original contact-dispatch parity remains open.
The v0.27 preview corrects homing cancellation and source fallback expiry presentation to ordinary fall (animation15), replacing the incorrectly forced bounce-fall loop (68). The preserved v0.26 package predates this correction.
The v0.27 preview also replaces empty-selection Light Speed Attack forced downward velocity with recovered no-target forces and the original failed-pursuit exit. Source terrain runs collision and reconciliation on the same frame; external hosts receive movement through World. The final hit remains in the attack until next-frame selection fails. Invalid target-volume cancellation retains the host validation policy. This change is included in v0.27.

A supported Light Speed Attack exit retains action10/fall presentation for its collision frame and selects landing before the next tick physics. Completed source contact records, including support flags, survive that handoff. Reset discards the pending landing; invalid external-host collision output restores the last finite pose/velocity. These changes are included in v0.27.

### Original character draw selection (development after v0.27)

A renderer should query Player::source_draw_selection() separately from source_animation_clock(). Selection::SpinModel substitutes model6/motion30 without changing the gameplay animation clock; SourcePose::sample_spin_override(active_animation, frame, action, lean) validates the frame against the active animation and samples the replacement motion without wrapping it to eight frames. Selection::Transition retains the actual prepared transition and its model. Selection::Current retains the active model/motion. Normal SourcePose::sample keeps its original frame limits.

The current adapter tracks draw-counter phases for charge, roll, Light Speed Attack and homing. Homing preserves the counter across entry and target loss; initial targetless/upward fallback preserves previous draw flags rather than automatically enabling the curled form. Ordinary jump takeoff leaves this override disabled. These presentation rules do not establish complete action-dispatch equivalence or GPU shading parity. Use the selection API each frame; caching only the gameplay animation ID can miss a model change.

Bounce also preserves and advances the draw counter while its curled form is
active. The source rebound apex selects67 and clears the override; repeated
bounce enables it again without resetting the counter. A subsequent homing
attack inherits this phase and the relevant draw flag. This source change
postdates the packaged v0.28 preview.

Rebound-to-homing has an action-specific exception: original action70 clears
flag100 for targetless/upward startup; active target initialization sets500.
The draw counter remains preserved. This supersedes the broad flag-inheritance
wording above for BounceUp. The separate BounceDown cancellation mapping is
not covered by the action70 reference probe.

The action69 downward-bounce branch8052c..8056c is now independently verified
against the same startup matrix as action70: both clear100 for fallback,
set500 for active targeting, and preserve360. Both use this policy in Player.
This supersedes the earlier limitation on downward-bounce startup coverage;
input eligibility and the full original dispatcher remain separate concerns.

Development after v0.29: pending source bounce contact takes priority over an
ordinary airborne A press. Action69 rebounds and action70 lands before homing
can interrupt; unsupported airborne bounce can still enter homing. The adapter
uses retained source support bits, not the public grounded flag alone.

During unsupported rebound, simultaneous A+B selects homing when available;
otherwise B can repeat the bounce. Pending contact still precedes both inputs.
This follows the original action70 ordering and is covered by source-record
sequence tests. This change postdates the packaged v0.29 preview.

The same simultaneous A+B priority applies during ordinary Jump: available
homing wins over bounce, as in original action6. The ground takeoff path still
consumes the initial press; this rule applies to the airborne follow-up press.

Development after v0.30: held-jump eligibility reads retained source action6,
including the descending animation phase. Original action10 walk-off/cancel
records remain ineligible. The existing held-update budget and source boost
formula are unchanged; the previous presentation-StateJump filter was too narrow.

### Development after v0.30: charged attack voice
The desktop audio preview now plays the verified English bank3/program8 voice once per LightAttackStart alongside its existing effect. voice-manifest.json contains independently decoded English and Japanese WAVs; the desktop adapter currently selects English. All loaded one-shots, including this voice, share pause/reset stopping. The renderer-independent ActionEvent contract is unchanged. A host should apply its own voice/language policy when consuming this event; original SA2 level31 and global voice gates are documented in reference/sonic-audio-cues.md and are not assumed to have equivalents in another game. Modern gain, resampling, scheduling and audibility remain outside the original-code equivalence claim.

### Development after v0.31: slope landing braking
Ordinary jump/fall contact now evaluates the recovered reverse-input landing brake on slopes as well as flat ground. Uphill and downhill source-terrain contact tests cover the transition. Stationary landing animation selection remains restricted to flat surfaces pending further original-context verification. This change does not establish arbitrary surface-transition or special landing parity.

### Development after v0.31: ordinary stationary slope landing
The ordinary jump/fall landing adapter now runs the recovered stop, surface-idle, orientation transfer and impact animation sequence on slopes as well as flat ground. It supplies the controller frame and idle RNG state. The bounded Rust sequence matches all entity, motion and character bytes in 18 original-executable cases. Those cases use supplied collision records; actual ramp contact tests separately verify leaving the airborne state. Special posture/action contexts and the hard-landing secondary effect remain outside this adapter. Bounce landing still uses its earlier helper pending a separate caller comparison. This supersedes the earlier flat-only stationary-landing limitation for ordinary jump/fall.

### Development after v0.31: rebound landing
Bounce recovery now calls the recovered rebound stop/idle/orientation/impact sequence with controller frame and idle RNG state. Unlike ordinary landing, this branch retains the idle helper counter update. Attack flags are cleared afterward. Eighteen original-executable record comparisons verify the bounded helper; a complete playable bounce/rebound/landing sequence verifies counter retention and attack-flag cleanup. Special posture/action contexts and hard-landing secondary effects remain incomplete. This supersedes the earlier note that bounce still uses the old stationary helper.

### Development after v0.31: moving landing continuation
When ordinary or rebound landing does not select a stationary animation, the controller now runs the recovered moving branch: transfer contact pitch/roll, select action1, then update locomotion using three-update heading history. Thirty-six original whole-record comparisons cover both callers on supplied flat/sloped records. Complete playable ordinary and bounce contacts verify the resulting ground state, orientation and locomotion selection. This does not establish arbitrary moving-platform or special-action behavior.

### Source collision eligibility API
The runtime exposes original_attack_pair::classify_sonic_contact for an already-overlapping pair. Supply the source entity (64 bytes), character (928 bytes), and both 48-byte collision descriptors plus receiver category0..3. It returns Sonic's updated classification/descriptor flags and whether the original pair rule marks the receiver. Inputs remain unchanged. Unsupported categories return an error; the API does not assume an unknown category is attackable. Shape overlap, hit registration, health, death, callbacks and repeated-contact handling belong to the host. The demo uses this API for roll, bounce, homing and charged attack eligibility; its one-hit target defeat policy remains authored. Lower-level classification and pair rules are compared against432 and16384 original cases respectively.

### Target profiles for roll/bounce
World::target_body_contact_profile supplies a receiver descriptor and category. The authored demo default is category2, defense0, receiver flag0x400. Roll/bounce overlaps now pass through source hit eligibility before emitting the existing one-hit TargetHit/defeated result. Source-terrain moves use retained source records; the host-physics compatibility path explicitly maps BounceDown to classificationE6 and Roll/BounceUp toE5 when no source records exist. That mapping is an adapter policy, not proof of full original action state. Unsupported categories and invalid records reject hits. Homing and charged-attack eligibility also consume these profiles, as described below. Host HP, repeated contact and enemy callback fidelity remain separate. This supersedes the earlier note that the demo does not use collision eligibility.

Post-v0.32 source contact bridge: source_body_contact also accepts matching retained homing action13 and Light Speed Attack action60 collision records. Missing or stale records return None. Actual source movement adapter tests distinguish strength1 homing from strength2 charged attack against defense1. The targeted defeat/rebound loops consume this eligibility as described below.

Targeted hit eligibility now consumes target_body_contact_profile for homing and Light Speed Attack too. Matching live records take precedence. Early contact without records and host-only physics use an explicit ordinary-Sonic classification (homing E5, light EE), without inferred powerups. Rejected damage does not invoke TargetHit or defeat the receiver. Existing homing contact response is preserved independently; original receiver HP/death and all response callbacks remain outside this bounded integration.

Category2/3 homing contacts with matching retained pursuit records now apply the recovered action6/flag/counter/animation transition and transfer those records into AirMotion without a takeoff impulse. The controller supplies its existing three-animation rebound cycle. Post-homing held-jump boost remains disabled. Six integration cases cover both categories/all cycles and the subsequent airborne update. Category0 orientation setter and host-only contact continuity remain separate.

Category0 source homing response now applies the complete recovered transition when matching pursuit records exist: direction angles, motion heading, world/local rebound impulse, action flags and animation cycle. AirMotion retains the result; controller velocity/yaw follow it. Forty-five original full-record comparisons cover the helper; nine controller cases cover categories0/2/3 through the next airborne tick. Host-only contact retains the explicit impulse-only adapter; source-terrain early contact prepares records as described below. Original invalid-angle integer sentinels are reproduced in the bounded math helper; this does not establish arbitrary special-gravity behavior.

Post-v0.33 development: source homing contact before the first pursuit movement now prepares the shared pursuit records without advancing motion. This removes the source-terrain early-contact impulse-only fallback for supported source responses. Eighteen early/delayed contact cases cover categories0/2/3 and all rebound cycles, unchanged contact position, and the next airborne frame. Worlds without source terrain retain their explicit adapter.

Post-v0.33 aura correction: Auras uses Player::source_air_action when retained airborne records exist. Original action6 emits snapshots during descent and recovered homing rebound even when presentation labels change. Grounded/stale records are excluded; host-only animation mapping remains explicit. This follows the source action6 effect call after collision and does not establish every effect scheduler ordering or framebuffer detail.

Post-v0.33 visual attachment: Player::effect_center rotates the local physics center height by the current model basis before translation. Desktop aura/dash use this center, and charge uses its midpoint with entity position. Eight original ordinary-action center cases match within 0.00001 host units, including rotated source airborne records. Attack-volume queries retain their existing separate contract. Host-only orientation and special-action collision centers remain adaptations.

Post-v0.34 attack centers now rotate the local source center height using matching pursuit, active bounce, ordinary airborne or grounded entity records. Action72 reverses that height as in the original common update. Missing/stale pursuit records retain the upright host fallback. Forty-eight original center snapshots and matching/stale pursuit tests cover this change; complete collision scheduling remains a separate fidelity boundary.

Early homing/charged overlap queries prepare missing pursuit records locally before testing the rotated center. This does not move Sonic or publish records; standalone attack_center still reports the documented upright fallback when matching records are absent. Target-selection and pursuit aim continue using their original entity-position rules, independently of collision-center placement.

Pursuit model orientation now consumes matching action13/60 collision records before considering older air/ground records. Aura/dash/charge attachment consequently follows the same pursuit rotation. Sixteen captured original pursuit center cases cover attachment and stale-action rejection; complete rendered fidelity remains broader than these comparisons.

Collision-center reference coverage now includes56original cases, adding actual bouncedown action69. Eight BounceMotion entry cases verify both attack and visual centers against original output. Current bounce uses actions69/70; the reversed action72 center is a separate original behavior and is not used by this bounce dispatcher.

Controller handoff: Snapshot.disconnected also signals replacement of an active XInput slot by a different connected slot. Snapshot.connected can therefore be true simultaneously. The desktop pauses and clears accumulated input on this signal; initial attachment does not signal a disconnect. Held buttons on a replacement do not create press edges. This has sampler-level tests; physical disconnect/reconnect acceptance remains open.

Clock elapsed-time contract: negative and nonfinite durations contribute zero elapsed time; finite durations are capped at0.1seconds per call. Existing accumulated fractions and pending input edges are preserved. A subsequent valid interval resumes normally. Call clear on pause/reset to deliberately discard accumulated time and input edges.

Active homing fallback model orientation takes priority over retained pursuit records. This also works when a host uses its own movement collision and no source-terrain records are present. Eight original action13 center cases cover fallback orientation with stale/absent pursuit records and a nonhoming control. No target acquisition or physics rule changes are implied.

Homing fallback expiry transfers its recovered action10 records into ordinary falling before the current physics step for both source terrain and host-owned collision. Host-only worlds no longer get a frozen expiry frame. Their move_player callback still owns collision; no new jump impulse is introduced. Both15fallback-update and360total-timeout boundaries have integration coverage.

B cancellation reads active fallback records directly before consulting pursuit terrain records. A host-owned collision world therefore retains the recovered action10 character/motion handoff even without source terrain. The cancelling press is consumed; a later fresh B press can bounce. Source target-contact priority remains unchanged.

Ordinary action6 aura snapshots use Player's center captured at the start of the fixed tick, before movement/common refresh. Auras::tick applies this automatically; callers continue supplying the current center for other effects. Reset clears the capture, and the first new tick captures the new spawn/teleport position. Evidence:24 original snapshot cases (16contact-positive), plus firstjump/repeatedjump/reset and 30/60/144FPS presentation tests. Out-of-band teleports should precede tick_in; direct presentation calls before any tick use their supplied center.

BounceDown aura snapshots also use the pre-movement center through aura_snapshot_center. This applies to both source-terrain and host-owned collision; direct un-ticked callers retain the suppliedcenter. The original bounce physics block has24snapshot cases including16selected-contact positives. Homing launch/dash/charge timing is not changed by this correction.

Homing launch presentation now retains a per-tick launch center independently of the final action state. Auras therefore emits the entry burst even when an immediate target hit rebounds during that same tick. tick_in clears this marker before each update and reset clears it. Launch uses pre-movement center; original special suppressionmask handling remains unverified.

Hard-landing secondary feedback is now identified as the original VibTask controller-vibration path (see reference/hard-landing-feedback.md in the development workspace). It is not an unidentified visual asset. Motor waveform, duration, settings and platform adapter remain unimplemented/unverified; landing-animation comparisons do not cover that feedback.

World::vibration(request,position) is an optional synchronous callback with a default no-op. Ordinary stationary landing emits original_vibration::request(6,15,0) when the recovered selector chooses animation17. Hosts own enable preferences, device identity and native motor mapping. No native output is currently wired. A source-collision test starts a real jump then injects hard/soft impact conditions; hard impact emitsonce and subsequentidle staysquiet. Directly setting airborne publicstate without a retained AirMotion record does not establish the recovered landing path.

feedback::Feedback is a platform-independent fixedtick scheduler. Queue normalizedrequests from World::vibration, tickonce persimulationupdate, route returned MotorCommand values through the host deviceadapter. clear discards pendingrequests and brakesactiveoutput forpause/reset/disconnect. Replacement resetscounter while retaining motorstate untiloriginalcommandboundary. Deferreddeletion is completedat hosttickend, an explicit adaptation; physical pulse parity is unverified. Native desktop integration remains pending.

Desktop feedback is now wired: PresentationWorld queues vibration requests and advances Feedback once per after_tick. The renderloop routes commands to XInputSetState on the selectedconnectedslot. Binary sourceStart maps toXboxleftmotor32767/right0; Stop/Brake map toboth0, anexplicit hardwareadaptation. Pause/reset/disconnect/slotreplacement clearfeedback andstoptheoldslot; MotorOutput Drop stopsnormalexit. Scriptedsmoke selectsNone andsendsno motorcommands. Physicalfeel, forcedprocesstermination andsystemleveldevicefailures remainunverified.

Stationary rebound landing also emits the normalized6/15/0 vibration request when recovered selection is17, matching original80080750. A natural input-only repeated-bounce sequence reaches the hardlanding and producesonefinitefeedbackpulse. Bounce-impact feedback itself hasnotbeenmapped bythischange; the callback coversfinalhardlanding only.

BounceImpact now also requests original7/15/0 feedback, matching800804c4. Both source-terrain Rebounded and host-owned collision bouncecount transitions emit onceperimpact. This supersedes earlier notes saying bounceimpact isunmapped. Natural repeatedbounce tests separate thetwoimpactpulses fromfinalhardlanding6/15/0; nativeamplitude remainsbinaryXboxadaptation.

### Host platform callback boundary (development after v0.40)
GroundMotion and AirMotion now expose resolve_source_terrain_with_platform with the original callback address, player-task address, entity address, slot address and mutable PlatformMemory. The existing resolve_source_terrain wrapper still rejects callbacks. Implementations must confine side effects to the supplied guest memory; the controller and collision state commit only on whole-step success, but external side effects cannot be rolled back. This exposes the recovered collision parent's callback path; it does not add moving platforms to DemoWorld or establish moving-platform gameplay fidelity. The packaged v0.40 build predates this API.
Callback boundary verification now includes attached-platform positive controls: six ground contacts and one flat airborne contact exercise argument forwarding, callback position edits plus slot translation, and failure rollback. The original static contact comparisons remain. The test callback is authored to exercise the API; it is not a recovered level-object callback. Airborne slope-platform contact, platform rotation over multiple ticks and jump-off behavior remain unverified.
Repeated collision updates now verify24 carry steps, a direction reversal and removal of the attachment without stale carry. These are collision-only updates, not a complete force-integrated ride. Jumping from retained attached contact is currently rejected by ordinary_jump_block with attached-object jump path not translated. Recovering that source branch is required before advertising jump-off support; clearing the attachment to bypass it is not an accepted implementation.

### Attached takeoff API and force lifetime
AirMotion::from_ground_with_platform accepts bounded read-only attachment memory and performs the recovered entry plus first physics pass. Supply the current player's records. Resolve collision next; after other frame consumers call finish_frame_forces before the next tick. This clears the verified external-force/displacement fields, not the whole original status/timer/history cleanup. Do not run tick again before the launch frame's collision: the constructor already performed its force step. The ordinary from_ground method still rejects attached context.

The host test now covers retained contact carry, launch with inherited impulse, twelve airborne updates without stale carry, descent to attached support, and resuming ground contact. This uses a synthetic attached flat triangle with authored slot translations, not a rendered moving mesh or a recovered level-object update. It proves adapter composition within that fixture, not general moving-platform or rotating-platform fidelity. DemoWorld and packagedv0.40 still lack moving platforms.

StaticTerrain::with_attachments(vertices, mappings) now accepts (triangle index, source task address) pairs. Zero addresses, duplicate mappings and out-of-range indices reject construction. The host must provide valid task/object/slot memory and preserve triangle ordering to retain identifiers across geometry snapshots. Construction alone does not animate a platform. A focused test translates triangle vertices across12 snapshots and updates the matching carry delta: grounded relative X remains constant, callbacks execute once per contact, and attached takeoff receives the positive motion impulse. World::source_terrain still requires a stable borrowed terrain; exposing changing snapshots through the playable World interface remains pending.
GroundMotion::jump_with_platform performs the same contact preparation and stick conversion as jump, then uses memory-aware takeoff. Both consume the ground controller and return an air controller whose first physics pass is already complete. Call collision next, then the documented frame-force clear. Supplying memory does not by itself enable attached terrain in Player::tick_in; the playable World snapshot/callback integration remains pending.

### Player ground/air platform integration
World::prepare_platform_memory updates source attachment memory before ground/ordinary-air contact. World::platform_callback implements the original indirect callback against transactional memory; default rejects unknown calls. source_terrain may return different precomputed snapshots across ticks, retaining triangle ordering/identifiers and one consistent snapshot throughout each tick. after_tick can select the next snapshot. Attached Player jumps use retained contact memory through GroundMotion::jump_with_platform. After frame observers, a nonzero transferred air force triggers the verified force clear and synchronizes retained collision records. This does not implement all source end-frame housekeeping.

The public Player::tick_in test rides24 updates of translated geometry, jumps using ordinary input, verifies12 airborne ticks without platform carry, lands and resumes carry. It exercises the real playable ground/air path. Moving-platform contact during bounce, homing and charged attacks still uses separate adapters and is not supported by these hooks yet. DemoWorld/rendering/package still have no visible moving platform. This supersedes the earlier statement that Player has no ground/air attachment integration.
BounceMotion::tick_with_platform now accepts memory preparation and the original callback. Both operate on the cloned next state; only successful physics commits it. The existing tick wrapper retains static behavior. Player routes World hooks through this path. Public moving-platform tests cover jump/bounce impact/rebound/landing and resumed carry on translated flat geometry. Homing/fallback and charged-attack platform contact are still separate unfinished paths; the earlier statement excluding bounce is superseded by this bounded test.
Platform hooks are now forwarded through AirFallback contact, shared attack contact and charged-exit collision as well. The runtime fallback static wrapper still rejects unknown callbacks. Public tests cover targetless homing returning to attached support with no respawn and full-charge targetless release recovery. These do not yet prove callback activation in every targeted homing/charged-attack contact branch. The homing fixture uses a larger triangle because the dash left the earlier small surface; respawn is explicitly excluded from its landing assertion.
Targeted platform coverage now drives public input through homing/category2 hit response and return to support, plus Light Speed Attack pursuit/hit/exit. Callback source actions60 and10 prove platform contact during charged pursuit and exit; targeted homing's recorded support callback is action6 after its aerial hit. These checks exclude respawn. A host selecting DemoUpward still owns the corresponding host movement/collision response; the moving-platform fixture explicitly selects SourceCategory2.


Historical limitation before v0.42 (superseded below): grounded charging still calls World::move_player even when source_terrain is provided. Development source now routes the initial short spin-dash release through recovered source collision, using the already integrated release records once; subsequent rolling frames use the same path. The v0.41 package predates this fix. Hosts still need the charging collision adapter. The continuous public-input move_chain test rejects host collision during Roll, including the release tick. Full original charge-frame and attachment equivalence remains unverified.


Development charge integration supersedes the charge collision limitation above: charging now retains GroundMotion records, applies original-compared action3 forces and resolves source terrain when supplied. The readiness transition uses ordinary ready locomotion. Host-only worlds still require move_player. A180tick test covers charging/readiness carry on the translating preview platform. v0.41 predates this integration; visual and physical acceptance remain open.


Controller fixes v0.43: ground B somersault, rolling B uncurl and held full charge through jumps. See [scope and source mapping](../reference/controller-fixes-v0.43.md). These fixes are a playtest increment, not full somersault contextual-dispatch parity.
