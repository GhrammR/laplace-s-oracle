//! The Cryptographic Calendar: Deterministic Planetary Cataclysms.
//!
//! Enforces:
//! 1. SHA-256 driven event triggers.
//! 2. Zero-allocation environment mutations.
//! 3. Perfect reproducibility of world-scale events.

use bevy_ecs::prelude::*;
use sha2::{Sha256, Digest};
use crate::physics::EnvironmentStack;
use crate::telemetry::Tick;

/// World Event System: Triggers cataclysms based on cryptographic hashes of the current tick.
pub fn world_event_system(tick: Res<Tick>, mut env: ResMut<EnvironmentStack>) {
    world_event_step(tick.0, &mut env);
}

pub fn world_event_step(tick: u64, env: &mut EnvironmentStack) {
    let mut hasher = Sha256::new();
    hasher.update(tick.to_le_bytes());
    let hash = hasher.finalize();

    // 1-in-a-million trigger using first 20 bits of SHA-256 hash
    let trigger_value = u32::from_le_bytes(hash[0..4].try_into().unwrap());
    
    if trigger_value < 4295 {
        // Deterministic Meteor Impact Coordinates
        let center_x = (hash[4] % 64) as usize;
        let center_y = (hash[5] % 16) as usize;

        eprintln!("[GREAT FILTER] Meteor Impact Event triggered at tick {} at ({}, {})!", tick, center_x, center_y);

        // Flip 4x4 block centered at (center_x, center_y)
        for dy in -2..2 {
            let row_idx = (center_y as isize + dy).rem_euclid(16) as usize;
            
            // Construct a 4-bit mask shifted to center_x
            // We'll use a simple loop for bit-flips to avoid complex wrapping logic for now,
            // as this is a rare event.
            for dx in -2..2 {
                let col_idx = (center_x as isize + dx).rem_euclid(64) as usize;
                
                env.temperature[row_idx] |= 1 << col_idx;
                env.particle[row_idx] |= 1 << col_idx;
            }
        }
    }
}
