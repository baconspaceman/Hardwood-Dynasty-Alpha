# Request 5: Faces, likeness and the optional face scan

## Goal
When the game gets graphics: use **generic placeholder faces** for every player, let the community
**add their own replacement faces**, and offer an **optional face scan** so a user can put
themselves in as their created player.

## Rules (read first)
- **The shipped game contains no real player likenesses.** No photos of real players, no scans of
  real players, no imagery taken from stat sites. Real players appear as name and ratings with a
  generic face until a user chooses to install a community pack.
- **Community packs are the user's responsibility.** Provide the plumbing and a clear notice in the
  UI and docs that replacement face packs are not part of the game and may carry their own
  licences; do not host or distribute them from the main repo.
- **Face scanning is a biometric feature.** Keep it **opt-in, on-device, and deletable.**
  Never upload a face, never include it in a shared save, mod or year pack by default, and never
  store raw photos. Explain exactly what is stored (a small face model or mesh) in plain English
  before the user starts. Treat anyone under 18 with extra caution: show an age-appropriate warning
  and a clear way to skip. Check the privacy and biometric-data laws that apply where the game is
  distributed before release.

## What to build
1. **A face data model** in the Rust library: a compact `FaceSpec` (skin tone, face shape,
   eyes/nose/mouth/hair/facial-hair choices, height/build tie-in, plus an optional reference to an
   external face asset id). Generated players get a deterministic `FaceSpec` from their id and
   origin/country pools (moddable lists, avoiding stereotypes: sample broadly and let the user edit).
2. **A placeholder face kit** in the client: a parametric head that reads `FaceSpec`, with enough
   variety that a roster does not look cloned. Original assets only, or openly licensed ones with
   the licence recorded.
3. **A face-pack format** for the community: a folder with a manifest (`name`, `author`, `licence`,
   `player_ids` or name keys, texture/mesh paths) that overrides placeholders. Validation reports
   problems in plain English. No pack is bundled.
4. **A face-scan flow (design + prototype).** Steps: consent screen, capture from a webcam or an
   imported photo, local processing (landmark detection to a parametric mesh and texture), preview
   and edit, save to a local file. Evaluate options such as on-device face-landmark libraries and
   photogrammetry-style approaches; document accuracy, licences, and runtime cost. The scan feeds
   the same `FaceSpec`/asset path as community packs. A "delete my scan" button removes it fully.
5. **Create-a-player integration.** In `hwd`/client, the created player can pick a placeholder face,
   a community pack face, or a local scan.

## Acceptance
- A seeded league shows varied placeholder faces; nothing resembling a real player.
- A sample community pack (made-up, original art) loads and overrides one player.
- The scan prototype works fully offline, writes only a local file, and deleting it leaves no trace.
- Written privacy notes in `docs/PRIVACY.md` (what is stored, where, how to delete it).
