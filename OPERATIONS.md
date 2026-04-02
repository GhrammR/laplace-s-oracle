# OPERATIONS.md: The Command Manual

This document is the definitive operator guide for running Laplace's Oracle, driving miracles through the Panopticon, and understanding the current orbital-era control surface.

## Section 1: The Trinity Protocol

### 1. Establish the Telemetry Pipe
```bash
mkfifo /tmp/oracle_pipe
```

### 2. Launch the Oracle Daemon
```bash
./laplace-oracle --interval 1 > /tmp/oracle_pipe
```

### 3. Launch the Panopticon TUI
```bash
./laplace-tui < /tmp/oracle_pipe
```

### 4. Dispatch the Genesis Miracle
```bash
./laplace-oracle --genesis --species 0x7FFF --x 32 --y 8
```

## Section 2: Standalone Oracle Flags

| Flag | Parameter | Description |
| :--- | :--- | :--- |
| `--interval` | `<u64>` | Telemetry broadcast frequency in ticks per frame. |
| `--genesis` | N/A | Dispatches a genesis event on startup. Requires `--species`, `--x`, and `--y`. |
| `--species` | `<u64>` | Taxonomic bitmask for the genesis event. |
| `--x` | `<u8>` | Target X coordinate for genesis. |
| `--y` | `<u8>` | Target Y coordinate for genesis. |
| `--archive` | N/A | Copies `universe.db` to a tick-stamped snapshot and exits. |
| `--seed-hash` | `<hex>` | Seeds the PRNG and signing key deterministically. |
| `--wormhole-rx` | `<socket-path>` | Binds a non-blocking Unix datagram socket for incoming migrations. |
| `--wormhole-tx` | `<socket-path>` | Stores the outbound Unix datagram target for ascension traffic. |
| `--scenario` | `<output.json>` | Runs the Oracle headlessly and writes a pretty-printed scientific summary JSON instead of emitting binary telemetry. |
| `--duration` | `<ticks>` | Required with `--scenario`; defines how many ticks the headless run should execute. |

The Oracle also binds the God-Mode API socket at `/tmp/oracle_api.sock` on startup and cleans it up on shutdown.

## Section 2A: Scenario Engine

For scientific extraction runs, execute the Oracle headlessly:
```bash
./target/release/laplace-oracle --scenario test_output.json --duration 100
```

Scenario mode runs fully in memory, suppresses binary telemetry on `stdout`, skips the live Panopticon path, and writes a pretty-printed JSON summary at the requested output path.

## Section 3: Telemetry and Visualization

The Panopticon verifies every frame signature before rendering. The current header exposes:
- current tick
- current celestial hour and orbital calendar state
- star type, season, and tide level decoded from `celestial_state`
- world hash verification status
- dropped-frame count
- Omega Sync percentage

The visualization layer cycle currently includes:
- `Biomass`
- `Water`
- `Temperature`
- `Structure`
- `Particle`
- `Pressure`
- `Microbiome`
- `Memetics`
- `Logic`
- `Light`
- `Geology`
- `Culture`

## Section 4: Panopticon God-Lens Commands

Press `:` in the Panopticon to enter command mode. Unless coordinates are explicitly accepted by the command, miracles are dispatched relative to the current cursor position.

### Navigation & View Control
- `Arrow Keys` or `hjkl`: move the cursor
- `Space`: cycle the primary visualization layer
- `Shift + Space`: cycle the reference overlay layer
- `q`: quit the Panopticon session

### Miracle Commands

| Command | Parameters | IPC Type | Effect |
| :--- | :--- | :--- | :--- |
| `/genesis` | `[mask] [x] [y]` | `Genesis` | Spawns a new population cluster and seeds biomass at the target. |
| `/fire` | `[radius]` | `Fire` | Sets temperature bits and burns biomass inside the brush. |
| `/rain` | `[radius]` | `Rain` | Adds water bits inside the brush. |
| `/build` | `[radius]` | `Build` | Sets structure bits inside the brush. |
| `/flood` | `[radius]` | `Flood` | Saturates water more aggressively across the brush. |
| `/drought` | `[radius]` | `Drought` | Clears water in the brush. |
| `/infect` | `<hash>` | `Infect` | Writes a memetic hash payload at the cursor cell. |
| `/logic-set` | `[radius]` | `LogicSet` | Sets logic bits inside the brush. |
| `/logic-clear` | `[radius]` | `LogicClear` | Clears logic bits inside the brush. |
| `/light-set` | `[radius]` | `LightSet` | Sets light bits inside the brush. |
| `/eclipse` | `[radius]` | `Eclipse` | Clears light bits inside the brush. |
| `/memetics-set` | `<hash> [radius]` | `MemeticsSet` | Writes the provided memetic payload across the brush. |
| `/memetics-clear` | `[radius]` | `MemeticsClear` | Clears memetic payloads across the brush. |
| `/pressure-set` | `[radius]` | `PressureSet` | Sets pressure bits inside the brush. |
| `/terrain-raise` | `[radius]` | `TerrainRaise` | Increments `elevation` by `+1` per cell in the brush, clamped to `255`. |
| `/terrain-lower` | `[radius]` | `TerrainLower` | Decrements `elevation` by `-1` per cell in the brush, clamped at `0`. |
| `/excavate` | `[radius]` | `Excavate` | Clears structure bits in the brush. |
| `/pause` | none | `Pause` | Pauses the simulation loop. |
| `/play` | none | `Play` | Resumes the simulation loop. |
| `/speed` | `<milliseconds>` | `Speed` | Sets the loop sleep interval in milliseconds. |

### Command Examples
```text
:/logic-set 3
:/memetics-set 0xDEADBEEF 2
:/terrain-raise 4
:/speed 8
```

## Section 5: Orbital Era Notes

The current light layer is not a static overlay. It is generated by `orbital_system` using the tick-zero celestial seed, an integer sine lookup table, and packed orbital state. Practical consequences for operators:
- `Light` is now seasonally and vertically mobile.
- `Pressure` receives tidal bonuses on the moon row.
- `Water` can be pulled laterally by the tide mask.
- Biomass birth remains coupled to active light bits.
- Manual `/light-set` and `/eclipse` miracles temporarily override the current illumination pattern at the targeted cells.
- `Elevation` is now dynamic rather than static: `tectonic_system` runs every 100 ticks, applying uplift where heat and pressure coincide and erosion where water sits above lower neighboring terrain.
- `Geology` is a compressed 2.5D layer: `^` marks magma vents and `.` marks solid crust in the Panopticon geology view.

## Section 6: Codex Release Discipline

Before a release:
1. Execute `/sync-docs` in Codex to refresh this file and `README.md` from the code.
2. Run `/audit`.
3. Execute `/release <version>`.

The governed release script will warn if `/sync-docs` has not been run, but the Codex agent is constitutionally required to perform the documentation sync before every release.
