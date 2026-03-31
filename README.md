# Laplace's Oracle: The Deterministic Simulation Engine

> "Everything is a result of a cause, and everything is a cause of a result."

Laplace's Oracle is a high-performance, strictly deterministic cosmological simulation engine designed to explore emergent complexity from bitwise cellular automata. It operates on the principle that if the initial state of the universe is known, every subsequent tick is mathematically inevitable.

## The Manifesto: Architectural Axioms

This project rejects the "Agentic Slop" of modern probabilistic AI in favor of rigid, verifiable, and deterministic logic.

1.  **The 8GB Law**: Maximum active RAM footprint is capped at 100MB. We do not allocate memory on the heap for primary data structures.
2.  **Memory Asceticism**: Zero-copy, memory-mapped I/O via `memmap2` and `rkyv`. The OS kernel remains the only memory manager.
3.  **Structural Determinism**: No random numbers unless seeded. No regex. No heap-based collections like `HashMap`. Everything is a sparse set or a bitgrid.
4.  **Process Sovereignty**: The simulation (Oracle) and the visualization (Panopticon) are separate processes linked by a cryptographic telemetry stream.
5.  **Cryptographic Provenance**: Every frame is signed with `ed25519-dalek`. If the signature doesn't match the world hash, the frame is compromised.

## The BitGrid Physics Engine

The simulation environment is represented by a multi-layered `EnvironmentStack`, a bitwise substrate where each bit is a discrete physical unit.

- **Biomass**: Organic matter and life potential.
- **Water**: Hydrated regions and flow dynamics.
- **Temperature**: Ignition points and thermal spread.
- **Structure**: Permanent physical barriers and constructions.
- **Particles**: Atmospheric matter and volcanic ejecta.
- **Microbiome**: A Conway's Game of Life substrate driving mutagenic evolution.
- **Memetics**: A 64-bit hash layer tracking the spread of cultural information.

## Getting Started

To run the simulation, refer to the [OPERATIONS.md](OPERATIONS.md) manual for the full **Trinity Protocol** setup.

---

*Laplace's Oracle is a project for those who value the precision of the machine over the ambiguity of the agent.*
