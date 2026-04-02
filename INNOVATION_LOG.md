# Innovation Log

## v0.10.0 - 2026-04-02
- Hypothesis: repeated `/logic-set` and `/pressure-set` interventions along existing structure corridors will expose stable emergent clocked computation regions, revealing whether the current NAND substrate can sustain operator-seeded signal loops.
- Suggestion: compress miracle application into shared bitwise brush helpers in `src/ipc.rs` so future atmospheric, pathogen, and mineral miracles can reuse one radius kernel instead of duplicating coordinate loops.
- Hypothesis: combining `/terrain-raise`, `/light-set`, and `/memetics-set` at the same frontier should create deterministic culture refugia where elevation, photosynthesis, and ideology reinforce one another into persistent apex niches.

## v0.11.0 - 2026-04-02
- Hypothesis: high-tide rows aligned with steep elevation gradients will create repeating littoral biomass collapse and recovery bands, producing deterministic coastal succession cycles without any new ecology substrate.
- Hypothesis: star-type-dependent daylight widths will expose a threshold where red-dwarf worlds preserve pressure-rich dark oceans while blue-giant worlds overdrive evaporation into self-sustaining atmospheric instability.
- Meta-Improvement: add a dedicated Codex skill `orbital-lab` that can run `/audit`, dump decoded celestial state at selected ticks, and compare deterministic orbital traces across branches so Tier 1 work stops relying on manual telemetry inspection.

## v0.12.0 - 2026-04-02
- Hypothesis: direct operator control over `pressure`, `light`, and `elevation` combined with orbital tides will let researchers induce deterministic monsoon corridors, revealing whether coastal civ-density spikes can be reproduced without adding any new climate layers.
- Hypothesis: recurrent `/sync-docs` refreshes anchored to the code surface will reduce future operator error rates, especially around miracles like `/terrain-raise`, `/logic-clear`, and orbital `Light` interactions whose semantics are easy to misremember.
- Meta-Improvement: add a dedicated Codex skill `ops-drift-audit` that compares `README.md`, `OPERATIONS.md`, `.codex/commands/*.md`, and the live command/parser surface, then emits a pre-release diff summary before `/release` is allowed to proceed.

## v0.13.0 - 2026-04-02
- Hypothesis: repeated tectonic uplift beneath pressure-rich orbital tide bands will create deterministic volcanic coastlines where `geology` vents and elevated basins lock into long-lived habitable crescents.
- Hypothesis: coupling erosion to lateral water flow and epoch-scaled uplift will produce naturally migrating river canyons even without a full voxel depth stack, letting `geology` act as a compressed proxy for underground instability.
- Meta-Improvement: add a Codex skill `deep-time-audit` that samples terrain snapshots every 100 ticks, compares `elevation` and `geology` diffs, and flags whether tectonic epochs are materially changing the world or wasting simulation budget.

## v0.14.0 - 2026-04-02
- Aha!: the cleanest scenario engine did not require a parallel simulation stack. Reusing the exact live Bevy schedule with an anonymous mmap and no Observation systems preserved determinism while eliminating stdout telemetry, which means research-mode and runtime-mode now share one physics path instead of drifting into separate engines.
- Hypothesis: batching many `--scenario --duration N` runs against fixed seeds and comparing only the final `world_hash`, biomass, and technology-bit outputs will expose hidden bifurcation thresholds in orbital, geological, and memetic coupling long before a full Python SDK exists.
- Hypothesis: because the scenario report decodes the apex taxonomy at the end of a headless run, we can detect lineage lock-in across seeds cheaply and use that as a selection signal for future Monte Carlo survival tournaments.
- Meta-Improvement: add a Codex skill `scenario-sweep` that runs multiple seeded `--scenario` experiments, stores the pretty JSON reports under `.artifacts/scenarios/`, and emits a compact comparative table before release candidates are approved.
