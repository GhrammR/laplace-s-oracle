# Laplace's Oracle: The Deterministic Simulation Engine

> "Everything is a result of a cause, and everything is a cause of a result."

Laplace's Oracle is a deterministic cosmological sandbox built on fixed-size bitwise substrates, signed telemetry, and memory-mapped operator control. The Oracle process advances the world, the Panopticon renders it, and every verified frame is cryptographically tied to the current world hash.

## The Manifesto: Architectural Axioms

This project rejects probabilistic orchestration in favor of reproducible, inspectable state transitions.

1. **The 8GB Law**: Maximum active RAM footprint is capped at 100MB. Heap growth for primary data structures is forbidden.
2. **Memory Asceticism**: Primary state lives in fixed-size substrates and memory-mapped files instead of per-entity heap objects.
3. **Structural Determinism**: No unseeded randomness, brittle regex mutations, or floating-point orbital math in the simulation path.
4. **Process Sovereignty**: The Oracle and the Panopticon are separate processes linked by a signed telemetry stream.
5. **Cryptographic Provenance**: Every frame is signed with Ed25519, and compromised signatures invalidate the frame.

## The BitGrid Physics Engine

The simulation world is a toroidal `64 x 16` `EnvironmentStack`. Each layer is fixed-width and evolves through deterministic cellular rules or direct operator miracles.

- **Biomass**: Organic matter, life potential, and computation fuel.
- **Water**: Hydrologic substrate for flow, flooding, precipitation, and tides.
- **Temperature**: Fire, heat transport, and evaporation triggers.
- **Structure**: Built or geological barriers, wire geometry, and excavation targets.
- **Particle**: Volcanic ejecta and advected particulate matter.
- **Pressure**: Atmospheric force, vortex formation, tides, and lowland retention bias.
- **Microbiome**: Conway-style microbial substrate driving mutagenic pressure.
- **Logic**: Bitwise computation substrate for wires, NAND gates, and semiconductor pulses.
- **Light**: Orbital illumination band used for photosynthesis, heating, and celestial drift.
- **Elevation**: Static terrain height map influencing fluid flow and pressure retention.
- **Memetics**: Per-cell cultural hash substrate for ideology and social diffusion.

## Orbital Mechanics & Celestial State

The light substrate is driven by deterministic orbital mechanics rather than linear drift. At tick zero, the Oracle derives a `CelestialSeed` from the `WorldHash`. That seed defines star type, axial tilt, orbital period, and moon phase offsets. Runtime orbital state is then computed through an integer sine lookup table in `src/math.rs`, which guarantees cross-platform determinism.

The resulting celestial frame drives:
- orbital light-band position and thickness
- packed telemetry state for star, season, tide, and day
- tidal pressure bonuses and lateral water pull on the moon row
- Panopticon header state such as `Star`, `Season`, and `Tide`

## Operator Surfaces

- **Panopticon TUI**: live visualization, cursor targeting, and slash-command miracles backed by `miracles.db`
- **God-Mode API**: non-blocking Unix socket ingress at `/tmp/oracle_api.sock`
- **Wormholes**: signed Unix datagram migration between Oracle instances
- **Codex CI Commands**: `/audit`, `/sanity-check`, `/release`, and `/sync-docs`

## Getting Started

For launch, control, and operator-command details, use [OPERATIONS.md](OPERATIONS.md).
