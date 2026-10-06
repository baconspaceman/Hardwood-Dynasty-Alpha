# Request 4: Design for the 3D game client

## Goal
A **design document and a small proof-of-concept**, not the full game. Decide the engine, how it
talks to the Rust simulation, and how the on-court experience works. The headline idea: you play
**as one player**, with an **over-the-shoulder camera and a limited field of view**. You should
not see the whole court the way you do in 2K. Awareness comes from what a real player would see
and hear, plus teammates and coaching cues.

## Deliverables
1. `docs/CLIENT_DESIGN.md` covering everything below.
2. A proof-of-concept project in `client/` (engine chosen below) that opens a court, moves ten
   placeholder players using data from the Rust engine, and shows the camera behaviour.

## Questions the design must answer
**Engine choice.** Compare Unreal, Unity and Godot for this project: licensing and fees, 3D
character animation tooling, networking if online play comes later, how easily each calls into
Rust (native plugin / GDExtension / C ABI), mod friendliness, build size, team skill. Recommend one.
(Godot is the project's history and has a first-class Rust binding, Unreal and Unity have stronger
character and animation pipelines. Make the call with evidence.)

**Rust integration.** The library stays UI-agnostic. Define a stable boundary: a C ABI or a
message protocol where the client sends inputs and receives world state each tick. Decide who owns
the clock. Keep the deterministic possession-level engine for league simulation, and add a
**real-time on-court layer** for played games (see below). Say how a real-time game result feeds
back into box scores, injuries and fatigue so a played game and a simmed game are comparable.

**Real-time on-court layer (new Rust module).** Positions, velocities, collisions, ball physics,
shot arcs, rebounds, passing lanes, fouls and contact, driven by the same ratings and tendencies as
the sim. Specify the update rate, determinism strategy, and how ratings change what is possible
(a 50 and a 99 passer feel different but both can pass; see request 1).

**The camera and "you are the player" view.**
- Over-the-shoulder follow camera on your player, with a modest field of view.
- No full-court radar by default. Awareness is earned: head-turn / look-around, a short-lived
  peripheral cue, teammate calls ("backdoor!"), a coach voice, court-side crowd noise.
- An optional **broadcast view** (side-line, TV style) for replays, spectating, and coaches/GMs.
- Player **lock-on**: switching players on defence, or coach/GM modes that use broadcast view.
- A settings group so accessibility and difficulty can widen the view (field of view, cue strength,
  a minimap toggle for people who need it). Every setting explained in plain English.
- Camera presets and a moddable camera config.

**Controls.** Controller first, keyboard second. Map: move, sprint, dribble moves, pass, shoot with
timing, post moves, defensive stance, steal/contest, call plays, switch player. Describe the input
model and how ratings gate moves.

**Animation and presentation.** Placeholder low-poly bodies and generic faces for now (see
request 5). Plan motion capture or a licensed animation set, skeleton standard, and retargeting so
community replacements work. Court, arenas, crowd, sound, commentary hooks.

**Modding.** How custom courts, jerseys, faces, animation packs and camera presets plug in.

**Performance and platforms.** Target frame rate, PC first, console later, minimum specs.

## Acceptance for the proof of concept
- Ten placeholder players move on a court driven by Rust state; the camera follows one player over
  the shoulder with a limited view; a toggle switches to a broadcast camera.
- A short README on how to build and run it on Windows.
