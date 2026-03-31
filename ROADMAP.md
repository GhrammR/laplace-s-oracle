# Laplace Oracle R&D Roadmap

This roadmap outlines the path towards a fully integrated, interactive biological and physical simulation, addressing identified logical disconnects and un-integrated logic paths.

## 1. Audit Summary: Identified Gaps
- **Population-Biomass Disconnect**: `Population` growth is purely procedural and lacks feedback from the `Biomass` bitgrid density.
- **Missing Structural Feedback**: `Action::Build` is undefined, and the `Structure` bitboard remains static/unwritten.
- **Thermal Lethality Absence**: `Temperature` (fire) consumes `Biomass` but does not trigger entity despawning or evacuation logic.
- **Static Neural State**: `SimHashBrain` signatures are fixed post-mutation, preventing behavioral adaptation or learning during an entity's lifespan.

---

## 2. Integration Roadmap

### STAGE 1: Malthusian Bitgrid Coupling
**Architectural Goal:** Link civilizational population dynamics to the availability of biological resources in the environment.
**Mechanism:** Bitwise density-dependent scaling of the Gillespie birth/death rates.
**Implementation:**
- Modify `TauLeap::step` in `src/temporal.rs` to accept `EnvironmentStack` as a parameter.
- Implement `biomass_density(pos, radius)` to query the bitboard.
- Scale `l_birth` and `l_death` based on local bit-occupancy.

### STAGE 2: Action::Build & Structural Persistence
**Architectural Goal:** Enable entities to modify the physical substrate by constructing persistent structures.
**Mechanism:** Hamming-distance mapped `Action::Build` that writes to the `Structure` bitgrid.
**Implementation:**
- Add `Action::Build` to the `Action` enum in `src/intelligence.rs`.
- Update `Intelligence::decide` to map specific distance ranges to `Build`.
- Implement `structure_system` in `src/physics.rs` to set bits in `env.structure` at entity coordinates.

### STAGE 3: Thermal Lethality & Evacuation
**Architectural Goal:** Integrate thermodynamic hazards into the entity survival model.
**Mechanism:** Spatial intersection check between entity `Position` and the `Temperature` bitboard.
**Implementation:**
- Create `hazard_system` in `src/physics.rs`.
- System iterates over entities and checks `env.temperature` at `(x, y)`.
- Trigger `commands.entity(e).despawn()` if bit is set.
- (Optional) Add `Panic` action to trigger movement away from high-temperature gradients.

### STAGE 4: Neural Plasticity & Learning
**Architectural Goal:** Allow entities to adapt their cognitive signatures based on environmental stimulus history.
**Mechanism:** Incremental bit-flip mutation of `SimHashBrain` towards the last $N$ `Stimulus` vectors.
**Implementation:**
- Add `LearningResource` to track environmental "reward" signals.
- Modify `mutation_system` in `src/intelligence.rs` to run periodically (not just for `NewlySpawned`).
- Implement `adjust_brain(brain, target_stimulus)` to minimize Hamming distance over time.

### STAGE 5: Hydro-Biological Dependency
**Architectural Goal:** Complete the environmental feedback loop by integrating the `water` substrate.
**Mechanism:** Multi-layer CA where `Biomass` growth requires `Water` proximity.
**Implementation:**
- Update `biology::life_step` to read from `env.water`.
- Modify CA rules: `birth` is only possible if `water` bit is set in neighborhood.
- Implement `evacuation_system` for `water` spread in `physics.rs`.

### STAGE 6: Cryptographic World State Verification
**Architectural Goal:** Ensure all physical and biological transitions are verifiable via the telemetry stream.
**Mechanism:** Merkle Tree hashing of the `EnvironmentStack` into the `WorldHash`.
**Implementation:**
- Update `hash_update_system` in `src/main.rs`.
- Integrate `EnvironmentStack` bytes into the `Sha256` hasher.
- Update `TelemetryFrame` to include the verified hash.

---

## 3. Verification Plan
- **Well-formedness**: Validate that `ROADMAP.md` is parsable Markdown.
- **Gap Coverage**: Ensure all 4 identified gaps (Population, Structure, Temperature, Brain) are addressed in the stages.
- **Axiom Adherence**: Confirm all proposed mechanisms (bitgrids, CA, zero-copy) respect the 8GB Law and Memory Asceticism rules.
