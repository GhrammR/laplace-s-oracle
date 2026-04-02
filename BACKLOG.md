# Backlog

This file is the sole source of truth for all unimplemented research and gameplay systems beyond `v0.9.0`.

The current engine is deterministic and structurally disciplined, but it is not a 1:1 simulation of reality. The roadmap below focuses on adding missing state spaces without violating the 8GB Law.

All new systems must obey these implementation constraints:
- No per-entity anatomy blobs, personality structs, or element inventories at planetary scale.
- Prefer bitboards, compressed voxel bit-stacks, hash-derived traits, and procedural decoding from existing identifiers such as `Taxonomy`, `SimHashBrain`, `TechnologyMask`, `CivIndex`, and `WorldHash`.
- Expensive biological and sociological detail must be reconstructed on demand from hashes and local substrate state rather than stored eagerly.

## Tier 1: Planetary Physics

### 3. The Periodic Table & Atmospheric Matrices
Introduce compressed chemistry layers for major atmospheric and geochemical categories such as Oxygen, CO2, Methane, Sulfur compounds, inert gases, metals, organics, and reactive radicals.
- 8GB-compliant mechanism: Do not track 118 elements per cell. Group elements by reactive flag families and store only a small set of bit-layers and low-bit-density concentration masks. Decode specific local chemistry procedurally from the family bits plus heat, pressure, water, and substrate exposure.
- Scientific Value: Supports atmospheric collapse studies, combustion chemistry, toxic bloom analysis, industrial pollution experiments, and comparative planetology.
- Gameplay Value: Makes worlds feel materially different: methane haze planets, oxygen-rich infernos, sulfur deathworlds, toxic industrial basins, and chemically exploitable frontier zones.

### 4. The Carbon Cycle
Build a global loop linking biomass growth, respiration, decay, fire, volcanism, ocean uptake, and atmospheric greenhouse loading.
- 8GB-compliant mechanism: Use aggregate carbon bit-layers and rolling planetary counters instead of per-organism carbon accounting. Convert biomass loss and fire masks into carbon and oxygen deltas through deterministic transfer rules and climate accumulators.
- Scientific Value: Necessary for climate realism, long-run biosphere feedback, and runaway greenhouse or snowball-planet experiments.
- Gameplay Value: Players can accidentally engineer extinction through industrial carbon release or stabilize worlds through deliberate ecological stewardship.

## Tier 2: Complex Biology

### 5. Procedural Anatomy
Generate bone structure, muscle density, circulatory resilience, armor, skin, carapace, organ redundancy, and weak-point exposure directly from the `Taxonomy` mask instead of storing anatomical records on each entity.
- 8GB-compliant mechanism: Reserve anatomical meaning for selected bit ranges in `Taxonomy`. Decode anatomy lazily during combat, survival, pathology, or rendering. Cache only transient local results when absolutely necessary.
- Scientific Value: Allows comparative morphology, survivability analysis, and evolutionary tradeoff studies across species without exploding memory.
- Gameplay Value: Creates species that feel mechanically distinct: fragile flyers, armored burrowers, high-endurance herbivores, and predators with precise weak points.

### 6. Phenotypic Abilities
Support flight, water-breathing, tunneling, climbing, venom, camouflage, thermal tolerance, and locomotion styles as mathematical consequences of gene-mask activations.
- 8GB-compliant mechanism: Derive ability vectors from activated crossover bits and a compact phenotype decoder function. Avoid storing an ability list per entity; recompute from genetic masks plus environmental context.
- Scientific Value: Enables trait-environment interaction research, adaptive radiation studies, and selection-pressure analysis.
- Gameplay Value: Makes species expansion, escape behavior, migration, and combat far more varied, because movement and survival are no longer just position and raw population.

### 7. Epidemiology (The Viral Substrate)
Create an explicit pathogen substrate independent of `microbiome`, with mutation pressure, transmissibility, host penalties, immunity interaction, and outbreak cascades.
- 8GB-compliant mechanism: Represent pathogens as bitboards plus a compact mutation-lineage hash. Hosts do not carry full disease structs; infection outcome is derived from pathogen hash, species anatomy decoder, local immune resistance, and exposure masks.
- Scientific Value: Supports outbreak simulation, spillover research, mutation-pressure experiments, quarantine studies, and intervention analysis.
- Gameplay Value: Introduces plagues, quarantines, bio-warfare, immune specialization, pandemic collapse, and emergent medical races.

## Tier 3: Deep Sociology

### 8. SimHash Personality Matrix
Compute boredom, anger, fear, grief, zeal, and cohesion from XOR convolutions of `SimHashBrain` against event markers such as deaths, fires, famines, defeats, miracles, and memetic shocks.
- 8GB-compliant mechanism: Do not store emotional state histories per entity. Recompute mood vectors from brain hash, recent event bitfields, and local world markers during decision phases.
- Scientific Value: Makes agent psychology measurable and reproducible without moving to an expensive agent-memory architecture.
- Gameplay Value: Produces panic, mourning spirals, revenge cycles, stagnation, fanaticism, morale collapse, and miracle-driven hysteria.

### 9. Procedural Entity Stats
Define strength, endurance, fertility, intelligence ceiling, aggression bias, perception, and disease resistance as procedural outputs from `Taxonomy` plus deterministic individual variance from `WorldHash`.
- 8GB-compliant mechanism: Base stats are never stored as full structs across millions of entities. Compute them on demand from species mask plus nonce-derived variance and optionally cache per-tick in transient scratch space only.
- Scientific Value: Allows population-level diversity and selection without turning the sim into an RPG database.
- Gameplay Value: Makes elite lineages, weak populations, hardy frontier species, prodigy outliers, and specialist subtypes emerge naturally.

### 10. Resource Extraction & Economics
Add `Minerals`, `Fossil_Fuels`, soil richness, and industrial throughput substrates, then drive inter-faction trade matrices and production bottlenecks from overlapping `CivIndex` influence zones.
- 8GB-compliant mechanism: Store resources as substrate layers and faction exposure masks, not inventory objects per worker. Trade values are aggregated matrix flows between civs, computed from access overlap, transport geometry, and scarcity hashes.
- Scientific Value: Necessary for modeling industrialization, energy transitions, war economics, and collapse from resource depletion.
- Gameplay Value: Turns terrain into strategy. Players get mining empires, oil wars, supply crises, embargoes, and asymmetric economies.

### 11. Ideological Schisms
Allow `Memetics` to polarize into incompatible belief clusters that fracture a civilization into rival `CivIndex` branches, driving coups, revolts, insurgencies, and civil war.
- 8GB-compliant mechanism: Ideology is represented as hashed belief signatures and polarization scores, not essay-like doctrine trees. Faction splitting occurs when local memetic Hamming distance crosses a threshold against ruling alignment and cohesion masks fail.
- Scientific Value: Enables study of polarization, legitimacy decay, insurgency formation, regime fragmentation, and memetic instability in complex societies.
- Gameplay Value: Internal collapse becomes as dangerous as external invasion. Empires can die from doctrinal incompatibility instead of only military defeat.

## Tier 5: The WebGL Panopticon

### 13. WebGL Panopticon
Build a lightweight local webserver that serves a Canvas/WebGL front-end, consumes the binary telemetry stream through WebSockets, and renders colorized high-framerate bitboards without modifying the headless Rust simulation core.
- 8GB-compliant mechanism: Keep the Rust core authoritative and headless. Stream the existing signed telemetry frames outward, decode them once in the browser, and let the GPU handle color, compositing, and animation rather than duplicating world state in the Oracle.
- Scientific Value: Researchers gain a richer inspection surface for long runs, comparative overlays, and visual anomaly detection without distorting the deterministic simulation substrate.
- Gameplay Value: Players get smooth, colorized, high-framerate observability that makes climate, culture, computation, and future chemistry layers legible in real time.

## Immediate Design Principle for All Tiers
Every new system must answer this question before implementation:
"How does this add realism and strategic depth without turning the engine into a heap of per-entity structs?"

If the answer is not "bitwise substrate, procedural decoding, or compact hash-derived aggregation," the design violates the roadmap.
