//! Verifiable Telemetry Pipeline for the Laplace Oracle.
//!
//! BINARY PROTOCOL SPECIFICATION (276 bytes total):
//! 00-03: [u8; 4] (Sync: 0xAA, 0xBB, 0xCC, 0xDD)
//! 04-11: u64 (Tick)
//! 12-43: [u8; 32] (WorldHash)
//! 44-47: u32 (Population)
//! 48-79: [u64; 4] (TechnologyMask)
//! 80-83: u32 (CivIndex)
//! 84-595: [u64; 64] (EnvironmentStack)
//! 212-275: [u8; 64] (Ed25519 Signature)

#![allow(unknown_lints)]
#![deny(clippy::all)]
#![forbid(unsafe_code)]

use crate::physics::EnvironmentStack;
use crate::temporal::Population;
use crate::intelligence::{SimHashBrain, TechnologyMask};
use bevy_ecs::prelude::*;
use ed25519_dalek::{SigningKey, Signer};
use std::io::Write;

// ── Telemetry Frame ──────────────────────────────────────────────────────────

/// 800-byte Hardened Telemetry Frame.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct TelemetryFrame {
    pub sync: [u8; 4],
    pub tick: u64,
    pub last_tick: u64,
    pub world_hash: [u8; 32],
    pub pop: u32,
    pub tech_mask: [u64; 4],
    pub apex_species_mask: u64,
    pub stack: EnvironmentStack,
    pub signature: [u8; 64],
}

pub const SYNC_HEADER: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xDD];
pub const PAYLOAD_SIZE: usize = 9180; // Frame without sync and signature
pub const FRAME_SIZE: usize = 9248;   // Total transmission size

impl TelemetryFrame {
    /// Manual serialization to ensure strict byte alignment and zero-copy stability.
    pub fn as_bytes(&self) -> [u8; 9248] {
        let mut buf = [0u8; 9248];
        buf[0..4].copy_from_slice(&self.sync);
        buf[4..12].copy_from_slice(&self.tick.to_le_bytes());
        buf[12..20].copy_from_slice(&self.last_tick.to_le_bytes());
        buf[20..52].copy_from_slice(&self.world_hash);
        buf[52..56].copy_from_slice(&self.pop.to_le_bytes());

        for i in 0..4 {
            let start = 56 + (i * 8);
            buf[start..start+8].copy_from_slice(&self.tech_mask[i].to_le_bytes());
        }

        buf[88..96].copy_from_slice(&self.apex_species_mask.to_le_bytes());

        // EnvironmentStack manual packing
        // Layers 0-6: [u64; 16] (896 bytes)
        for i in 0..16 {
            buf[96 + i*8..96 + (i+1)*8].copy_from_slice(&self.stack.biomass[i].to_le_bytes());
            buf[224 + i*8..224 + (i+1)*8].copy_from_slice(&self.stack.water[i].to_le_bytes());
            buf[352 + i*8..352 + (i+1)*8].copy_from_slice(&self.stack.temperature[i].to_le_bytes());
            buf[480 + i*8..480 + (i+1)*8].copy_from_slice(&self.stack.structure[i].to_le_bytes());
            buf[608 + i*8..608 + (i+1)*8].copy_from_slice(&self.stack.particle[i].to_le_bytes());
            buf[736 + i*8..736 + (i+1)*8].copy_from_slice(&self.stack.pressure[i].to_le_bytes());
            buf[864 + i*8..864 + (i+1)*8].copy_from_slice(&self.stack.microbiome[i].to_le_bytes());
        }

        // Layer 7: Memetics [u64; 1024] (8192 bytes)
        for i in 0..1024 {
            let start = 992 + i * 8;
            buf[start..start+8].copy_from_slice(&self.stack.memetics[i].to_le_bytes());
        }

        // Signature (starts at 992 + 8192 = 9184)
        buf[9184..9248].copy_from_slice(&self.signature);
        buf
    }
}

// ── Resources ────────────────────────────────────────────────────────────────

#[derive(Resource, Default, Clone, Copy)]
pub struct LastTick(pub u64);

#[derive(Resource, Default, Clone, Copy)]
pub struct Tick(pub u64);

#[derive(Resource)]
pub struct TelemetryInterval(pub u64);

#[derive(Resource, Default)]
pub struct DroppedFrames(pub u64);

#[derive(Resource)]
pub struct SigningKeyResource(pub SigningKey);

#[derive(Resource, Default)]
pub struct WorldHash(pub [u8; 32]);

#[derive(Resource)]
pub struct StdoutResource(pub std::io::Stdout);

// ── Observation System ───────────────────────────────────────────────────────

/// System that packs and signs the current world state, emitting it as binary.
#[allow(clippy::too_many_arguments)]
pub fn observation_system(
    tick: Res<Tick>,
    mut last_tick: ResMut<LastTick>,
    interval: Res<TelemetryInterval>,
    mut dropped: ResMut<DroppedFrames>,
    stdout_res: Res<StdoutResource>,
    signing_key: Res<SigningKeyResource>,
    hash: Res<WorldHash>,
    stack: Res<EnvironmentStack>,
    q: Query<(&Population, &SimHashBrain, &TechnologyMask)>,
) {
    if tick.0 % interval.0 != 0 {
        return;
    }

    let mut apex_pop = 0;
    let mut apex_mask = 0;
    let mut tech_mask_sum = [0u64; 4];

    // Find the single entity with the highest population (the "Apex Species")
    for (p, brain, t) in q.iter() {
        if p.0 > apex_pop {
            apex_pop = p.0;
            apex_mask = brain.0;
        }
        tech_mask_sum[0] |= t.0[0];
        tech_mask_sum[1] |= t.0[1];
        tech_mask_sum[2] |= t.0[2];
        tech_mask_sum[3] |= t.0[3];
    }

    let mut frame = TelemetryFrame {
        sync: SYNC_HEADER,
        tick: tick.0,
        last_tick: last_tick.0,
        world_hash: hash.0,
        pop: apex_pop,
        tech_mask: tech_mask_sum,
        apex_species_mask: apex_mask,
        stack: *stack,
        signature: [0u8; 64],
    };

    // Sign the payload (everything between sync and signature)
    let buffer_tmp = frame.as_bytes();
    let data_to_sign = &buffer_tmp[4..9184];

    let sig_bytes = signing_key.0.sign(data_to_sign).to_bytes();
    frame.signature = sig_bytes;

    let final_buffer = frame.as_bytes();

    // 3. Emit binary (Atomic write_all)
    let mut stdout = stdout_res.0.lock();
    if stdout.write_all(&final_buffer).is_ok() {
        let _ = stdout.flush();
        last_tick.0 = tick.0;
    } else {
        dropped.0 += 1;
    }
}
