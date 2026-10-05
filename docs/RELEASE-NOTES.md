# v0.44.1 saved character demo

The new course includes a 48-unit-wide, 16-unit-deep skate bowl with an apron,
and a 24-unit-diameter, 90-unit-long round tunnel. Collision and rendering use
the same surfaces. Adequate speed supports full loops; losing speed detaches
Sonic. The tube camera and stick controls follow the contact orientation.

Xbox controls: left stick moves, A jumps/homing attacks. B or X is the SAME
contextual Action: a deliberate short press/release starts a somersault; hold
past 24 simulation ticks to charge, then release to spin dash. While rolling,
a fresh Action press uncurls. In the air it bounces. Near a ring in front it
starts Light Dash. Full Light Speed Attack readiness stays held through jumps
until Action is released. Y has no separate spin-dash binding.

Repeated deliberate Action presses queue the normal three-stage somersault
chain. Flame Ring is enabled, using the recovered original texture sequence
and particle initialization/update routines. Light Dash uses original target
selection, steering, timers and collision routines with demo ring contacts.
The optional audio preview includes its original looping sample.

MENU opens practice starts for the bowl, tunnel and ring row. In the tunnel,
point across its width, hold Action about one second, then release to build
looping speed. Let go of the stick to test momentum, or steer through the loop.
A jumps away from the contacted surface. Slow attempts should fall back down.
Right stick rotates the camera, triggers zoom, RS centers, VIEW toggles help.
Keyboard: WASD/arrows, Space jump, B Action, Escape menu, R restart, H help.

This is a playable milestone, not a complete engine port. Normal and high-speed somersault combos are integrated, including the
96-series opening repeat. Complete clearance-dependent repetition and
whole-dispatcher equivalence are not established. Special collision/reset ordering and higher-priority
interrupts during Light Dash are not fully reproduced. Ring overlap discovery,
particle scheduling/RNG stream, camera, lighting, gain and mixing are host
adaptations. The source renderer is required for the newly recovered poses;
it is included and selected automatically in this package.

Verification: current headless integration suite, native loop pause/reset test,
packaged asset/simulation checks and rendered fire/rings/bowl/inverted-tunnel
captures. The old fixed-frame --smoke script predates shared-button timing;
use --smoke OUTPUT --new-features for the current bounded visual check.
Physical controller feel and audible matching remain user playtests. Zelda
integration is still downstream. The confirmed left-stick build is preserved.

User acceptance: v0.44.0 was playtested and approved for saving/packaging.
v0.44.1 adds the final tested high-speed somersault sequence; its physical
controller feel has not separately been confirmed by the user.
