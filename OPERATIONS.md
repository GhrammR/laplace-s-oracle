# OPERATIONS.md: The Command Manual

This document is the definitive guide to running and interacting with Laplace's Oracle.

## Section 1: The Trinity Protocol
To launch the full simulation environment, follow these steps to establish the telemetry conduit and process hierarchy.

### 1. Establish the Telemetry Pipe
The Oracle daemon streams cryptographic telemetry via `stdout`. We use a named pipe to decouple the simulation from the visualization layer.
```bash
mkfifo /tmp/oracle_pipe
```

### 2. Launch the Oracle Daemon
Start the simulation engine. The daemon will output its Public Key to `stderr`; copy this string for the TUI.
```bash
./laplace-oracle --interval 1 > /tmp/oracle_pipe
```

### 3. Launch the Panopticon TUI
In a new terminal, launch the TUI, providing the Public Key from the daemon's startup output.
```bash
./laplace-tui <PUBLIC_KEY_B64> < /tmp/oracle_pipe
```

### 4. Dispatch the Genesis Miracle (God CLI)
The simulation is now running but empty. Use the God CLI to seed the first civilization.
```bash
./laplace-oracle --genesis --species 0x7FFF --x 32 --y 8
```

---

## Section 2: Standalone Oracle Commands
The `laplace-oracle` binary supports the following flags for lifecycle management and state manipulation.

| Flag | Parameter | Description |
| :--- | :--- | :--- |
| `--interval` | `<u64>` | Set the telemetry broadcast frequency (ticks per frame). Default: `1`. |
| `--genesis` | N/A | Trigger a Genesis event on startup. Requires `--species`, `--x`, and `--y`. |
| `--species` | `<u64>` | The taxonomic bitmask for the Genesis event. |
| `--x` | `<u8>` | The target X coordinate (0-63) for the Genesis event. |
| `--y` | `<u8>` | The target Y coordinate (0-15) for the Genesis event. |
| `--archive` | N/A | Snapshot the current `universe.db` to a timestamped file and exit. |
| `--seed-hash` | `<HEX>` | Provide a 32-byte hex string to seed the PRNG and ECDSA key. |

---

## Section 3: Panopticon God-Lens Commands
Within the Panopticon TUI, press `:` to enter command mode. All miracles are dispatched relative to the current **Cursor position** unless coordinates are specified.

### Navigation & Visualization
- **Arrow Keys / `hjkl`**: Move the targeting reticle.
- **`Space`**: Cycle the primary visualization layer.
- **`Shift + Space`**: Cycle the reference (ghost) layer for comparison.
- **`q`**: Terminate the Panopticon session.

### Miracle Commands
| Command | Parameters | Effect |
| :--- | :--- | :--- |
| `/genesis` | `[mask] [x] [y]` | Spawns a new population at the target or cursor. |
| `/fire` | `[radius]` | Ignites the temperature layer in the specified radius. |
| `/rain` | `[radius]` | Saturates the water layer in the specified radius. |
| `/build` | `[radius]` | Deposits structural matter in the specified radius. |
| `/infect` | `<hash>` | Injects a specific memetic hash at the cursor position. |

**Example Genesis Dispatch:**
```text
:/genesis 0x0F0F 10 5
```
*(Spawns an Insectoid colony at coordinates 10, 5)*
