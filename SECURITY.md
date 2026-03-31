# SECURITY.md: The Threat Model

This document outlines the security policy and threat model for Laplace's Oracle.

## State Integrity and Data Sovereignty

The state of the simulation is stored in `universe.db`. The integrity of this file is paramount to the determinism of the system.

- **Delegated Protection**: No internal encryption is applied to the state database to maintain peak I/O performance. Security and access control are delegated to **filesystem permissions**.
- **Bit-Perfect Snapshots**: The `--archive` command creates an exact copy of the state, ensuring that the provenance of a simulation can be preserved even if the live database is corrupted.

## Cryptographic Provenance

Telemetry frames dispatched by the Oracle daemon are protected against tampering via a strict cryptographic signature.

- **ED25519 Signatures**: Every frame contains a 64-byte Ed25519 signature. The signature covers the entire telemetry payload, excluding the sync bytes and the signature itself.
- **Verifying the Panopticon**: The Panopticon TUI requires the daemon's public key upon startup. Frames that fail signature verification are immediately dropped to prevent "Agentic Slop" or junk data from polluting the visualization.

## IPC and DoS Vectors

Laplace's Oracle uses a combination of memory-mapped files (`miracles.db`) and named pipes (`/tmp/oracle_pipe`) for inter-process communication.

- **Miracle Injection**: Any process with write access to `miracles.db` can dispatch miracle commands to the daemon. This is an intentional "God-Eye" feature.
- **DoS Risks**: The named pipe `/tmp/oracle_pipe` and the file `miracles.db` are potential Denial-of-Service vectors. An unauthorized process could flood these conduits with junk data, leading to buffer overflows or CPU saturation.
- **Accepted Risk**: In a local simulation context, this risk is accepted. It is recommended to run the Oracle processes under a dedicated service user with restricted permissions.

---

*Security is a function of determinism. An unpredictable system is an insecure system.*
