//! Verifiable Telemetry Pipeline for the Laplace Oracle.
//!
//! BINARY PROTOCOL SPECIFICATION (9280 bytes total):
//! 00-03: [u8; 4] (Sync: 0xAA, 0xBB, 0xCC, 0xDD)
//! 04-11: u64 (Tick)
//! 12-19: u64 (LastTick)
//! 20-51: [u8; 32] (WorldHash)
//! 52-55: u32 (Population)
//! 56-87: [u64; 4] (TechnologyMask)
//! 88-95: u64 (Apex Species Brain Mask)
//! 96-127: [u64; 4] (Apex Linguistic Sequence)
//! 128-9215: EnvironmentStack payload
//! 9216-9279: [u8; 64] (Ed25519 Signature)

#![allow(unknown_lints)]
#![deny(clippy::all)]
#![forbid(unsafe_code)]

use crate::intelligence::{LinguisticSequence, SimHashBrain, TechnologyMask};
use crate::physics::EnvironmentStack;
use crate::temporal::Population;
use bevy_ecs::prelude::*;
use ed25519_dalek::{Signer, SigningKey};
use std::io::Write;

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
    pub apex_linguistic_sequence: [u64; 4],
    pub stack: EnvironmentStack,
    pub signature: [u8; 64],
}

pub const SYNC_HEADER: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xDD];
pub const PAYLOAD_SIZE: usize = 9212;
pub const FRAME_SIZE: usize = 9280;

impl TelemetryFrame {
    pub fn as_bytes(&self) -> [u8; FRAME_SIZE] {
        let mut buf = [0u8; FRAME_SIZE];
        buf[0..4].copy_from_slice(&self.sync);
        buf[4..12].copy_from_slice(&self.tick.to_le_bytes());
        buf[12..20].copy_from_slice(&self.last_tick.to_le_bytes());
        buf[20..52].copy_from_slice(&self.world_hash);
        buf[52..56].copy_from_slice(&self.pop.to_le_bytes());

        for i in 0..4 {
            let start = 56 + (i * 8);
            buf[start..start + 8].copy_from_slice(&self.tech_mask[i].to_le_bytes());
        }

        buf[88..96].copy_from_slice(&self.apex_species_mask.to_le_bytes());
        for i in 0..4 {
            let start = 96 + (i * 8);
            buf[start..start + 8].copy_from_slice(&self.apex_linguistic_sequence[i].to_le_bytes());
        }

        for i in 0..16 {
            buf[128 + i * 8..128 + (i + 1) * 8]
                .copy_from_slice(&self.stack.biomass[i].to_le_bytes());
            buf[256 + i * 8..256 + (i + 1) * 8].copy_from_slice(&self.stack.water[i].to_le_bytes());
            buf[384 + i * 8..384 + (i + 1) * 8]
                .copy_from_slice(&self.stack.temperature[i].to_le_bytes());
            buf[512 + i * 8..512 + (i + 1) * 8]
                .copy_from_slice(&self.stack.structure[i].to_le_bytes());
            buf[640 + i * 8..640 + (i + 1) * 8]
                .copy_from_slice(&self.stack.particle[i].to_le_bytes());
            buf[768 + i * 8..768 + (i + 1) * 8]
                .copy_from_slice(&self.stack.pressure[i].to_le_bytes());
            buf[896 + i * 8..896 + (i + 1) * 8]
                .copy_from_slice(&self.stack.microbiome[i].to_le_bytes());
        }

        for i in 0..1024 {
            let start = 1024 + i * 8;
            buf[start..start + 8].copy_from_slice(&self.stack.memetics[i].to_le_bytes());
        }

        buf[9216..9280].copy_from_slice(&self.signature);
        buf
    }
}

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
    q: Query<(
        &Population,
        &SimHashBrain,
        &TechnologyMask,
        &LinguisticSequence,
    )>,
) {
    if !tick.0.is_multiple_of(interval.0) {
        return;
    }

    let mut apex_pop = 0;
    let mut apex_mask = 0;
    let mut apex_linguistic_sequence = [0u64; 4];
    let mut tech_mask_sum = [0u64; 4];

    for (p, brain, t, sequence) in q.iter() {
        if p.0 > apex_pop {
            apex_pop = p.0;
            apex_mask = brain.0;
            apex_linguistic_sequence = sequence.0;
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
        apex_linguistic_sequence,
        stack: *stack,
        signature: [0u8; 64],
    };

    let buffer_tmp = frame.as_bytes();
    let data_to_sign = &buffer_tmp[4..9216];
    frame.signature = signing_key.0.sign(data_to_sign).to_bytes();

    let final_buffer = frame.as_bytes();
    let mut stdout = stdout_res.0.lock();
    if stdout.write_all(&final_buffer).is_ok() {
        let _ = stdout.flush();
        last_tick.0 = tick.0;
    } else {
        dropped.0 += 1;
    }
}
