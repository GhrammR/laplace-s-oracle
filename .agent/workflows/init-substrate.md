---
description: init-substrate
---

Execute Stage 1: The Cryptographic Substrate & Zero-Copy ECS.

Adhering strictly to the Global Rule, generate a Rust application with a `main.rs` and `Cargo.toml`.

1.  Initialize a strictly headless `bevy_ecs` World.
2.  Define a custom storage backend for Bevy components that uses the `memmap2` crate to map a 1GB file named `universe.db` to the process's address space.
3.  Use the `rkyv` crate to define a `#[derive(Archive)]` component struct `StateVector { position: [f32; 3] }`. Ensure zero-copy deserialization from the memory-mapped file.
4.  Integrate the `ed25519-dalek` and `sha2` crates. Upon initialization, calculate the SHA-256 hash of the initial (empty) world state and sign this hash with a newly generated ed25519 keypair.
5.  Print the public key and the base64-encoded signature to the console.
6.  Omit all logging, comments, and placeholder code. Output only the final, compilable source.