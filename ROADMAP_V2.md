# ROADMAP V2 — The Cosmological Simulation Directive

> **Epoch:** 2026-03-30  
> **Subject:** 9-Month R&D Roadmap for Emergent Universal Complexity  
> **Constraint:** 8 GB Law, Zero-Allocation Axioms, Cryptographic Provenance  

---

## Quarter 1 — The Physical Universe

### Phase I (Month 1): Particle Advection & Gravity

**Goal:** Simulate basic Newtonian physics for all non-biological matter via bitwise advection.

| Change | Detail | Status |
|--------|--------|--------|
| `EnvironmentStack` | Add `particle: [u64; 16]` (fifth layer) | [COMPLETE] |
| `gravity_system` | Downward bit-shift; halted by `structure` | [COMPLETE] |
| `volcanic_eruption_system` | Critical heat + pressure → spawn `particle` | [COMPLETE] |
| Telemetry | Frame size updated to 928 bytes; unified protocol | [COMPLETE] |

**Verification:** `cargo check && cargo test`

---

### Phase II (Month 2): Atmospheric Dynamics & Weather

**Goal:** Simulate wind, pressure gradients, and catastrophic weather events.

| Change | Detail | Status |
|--------|--------|--------|
| `EnvironmentStack` | Add `pressure: [u64; 16]` (sixth layer) | [COMPLETE] |
| `wind_system` | High-pressure gradients push `particle` and `temperature` bits | [COMPLETE] |
| Tornadoes | 3x3 pressure vortex → rotational bit-shift destroys infrastructure | [COMPLETE] |

**Verification:** `cargo check && cargo test`

---

### Phase III (Month 3): The Cryptographic Calendar & World Events

**Goal:** Deterministic un-scripted planetary cataclysms via hash-based event triggers.

| Change | Detail | Status |
|--------|--------|--------|
| `world_event_system` | SHA-256 hash of `current_tick` each sim step | [COMPLETE] |
| Great Filter Event | If `trigger < 4295` (1-in-1M) → 4x4 Meteor Impact (heat + ash) | [COMPLETE] |

**Verification:** `cargo check && cargo test`

---

## Quarter 2 — The Biological Engine

### Phase IV (Month 4): The Full Genetic Algorithm

**Goal:** True Darwinian evolution through bitwise crossover and mutation.

| Change | Detail |
|--------|--------|
| `breeding_system` | Adjacent same-`Class` entities with sufficient `Biomass` spawn a child | [COMPLETE] |
| Crossover | `child.taxonomy = (p1 & top_mask) \| (p2 & bottom_mask)` | [COMPLETE] |
| Mutation | Random bit-flip on child's `SimHashBrain`; rate proportional to local `Temperature` | [COMPLETE] |

**Verification:** `cargo check && cargo test`

---

### Phase V (Month 5): The Expanded Taxonomic Decoder

**Goal:** Fulfill the "every species on Earth" mandate with a rich decodable biological space.

| Change | Detail |
|--------|--------|
| `decode_taxonomy` | Hundreds of named bit-pattern mappings (Human, T-Rex, Salmon, …) | [COMPLETE] |
| Procedural fallback | Unmapped patterns → `"Procedural Carnivora 7A3F"` style names | [COMPLETE] |

**Verification:** `cargo check && cargo test`

---

### Phase VI (Month 6): The Microbial Butterfly Effect

**Goal:** Demote microbes to an environmental substrate that drives evolution.

| Change | Detail |
|--------|--------|
| Remove `MICROBE` spawn | No longer a spawnable species | [COMPLETE] |
| `EnvironmentStack` | Add `microbiome: [u64; 16]` | [COMPLETE] |
| Microbiome CA | Independent cellular-automaton update rules | [COMPLETE] |
| Breeding coupling | Child on a `1` microbiome bit → additional targeted `XOR` mutation on `Taxonomy` | [COMPLETE] |

**Verification:** `cargo check && cargo test`

---

## Quarter 3 — The Sociological Layer

### Phase VII (Month 7): The Memetics Engine

**Goal:** Simulate the spread of ideas, culture, and societal structure.

| Change | Detail |
|--------|--------|
| `EnvironmentStack` | Add `memetics: [u64; 16]` |
| Meme discovery | Entities discover memes (64-bit hashes representing concepts) via tech-discovery system |
| Propagation | Entity interaction → meme `OR` crossover |
| Effect | Memes modify `SimHashBrain`, altering group behaviour (e.g. "Warfare" amplifies `spatial_conflict_system` fitness) |

**Verification:** `cargo check && cargo test`

---

### Phase VIII (Month 8): The Interactive God-Lens

**Goal:** Transform the Panopticon TUI from passive observer to interactive command console.

| Change | Detail | Status |
|--------|--------|--------|
| Modal Input | Toggle `Normal` vs `Command` modes with `:` / `Esc` | [COMPLETE] |
| CLI Miracles | `/genesis` command via `miracles.db` IPC | [COMPLETE] |
| Reactive Loop | Non-blocking `event::poll` for zero-latency interaction | [COMPLETE] |

**Verification:** `cargo check && cargo test`

---

### Phase IX (Month 9): The Archivist & The Next Universe

**Goal:** Universe persistence, versioning, and cyclical cosmology.

| Change | Detail |
|--------|--------|
| `--archive` flag | Copies `universe.db` → `universe.db.tick_<N>` |
| `--seed-hash <SHA256>` | Initialises a new universe using the final `WorldHash` of a previous one ("Big Crunch / Big Bang" cycle) |

**Verification:** `cargo check && cargo test`

---

## Autonomous Verification Protocol

After **every phase**, the following gate must pass before proceeding:

```bash
cargo check 2>&1 | grep -E "^error" && echo "GATE FAIL" || echo "GATE PASS"
cargo test -- --nocapture
```

The binary telemetry frame size seal test (`telemetry_frame_size_seal`) is re-anchored to the correct byte count after each phase that mutates `EnvironmentStack`.

---

*Status: Phase VI complete. The microbial world drives macro-evolution.*
