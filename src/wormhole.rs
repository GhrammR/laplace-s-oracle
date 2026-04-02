//! Wormhole transport substrate for Laplace Oracle.
#![forbid(unsafe_code)]

use crate::biology::Taxonomy;
use crate::intelligence::{Action, LinguisticSequence, NewlySpawned, SimHashBrain, TechnologyMask};
use crate::ipc::MiracleGrace;
use crate::telemetry::SigningKeyResource;
use crate::temporal::{CivIndex, Population, Position};
use bevy_ecs::prelude::*;
use ed25519_dalek::{Signature, Signer, Verifier};
use std::io::ErrorKind;
use std::os::unix::net::UnixDatagram;
use std::path::PathBuf;

pub const WORMHOLE_ACTIVITY_OUTGOING: u8 = 1;
pub const WORMHOLE_ACTIVITY_INCOMING: u8 = 2;
pub const WORMHOLE_ASCENSION_BIT: usize = 255;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WormholePayload {
    pub taxonomy: u64,
    pub brain: u64,
    pub linguistics: [u64; 4],
    pub tech: [u64; 4],
    pub population: u32,
    pub signature: [u8; 64],
}

impl WormholePayload {
    pub const BODY_SIZE: usize = 84;
    pub const SIGNATURE_OFFSET: usize = 84;
    pub const BYTE_SIZE: usize = 148;

    pub fn body_bytes(&self) -> [u8; Self::BODY_SIZE] {
        let mut buf = [0u8; Self::BODY_SIZE];
        buf[0..8].copy_from_slice(&self.taxonomy.to_le_bytes());
        buf[8..16].copy_from_slice(&self.brain.to_le_bytes());
        for idx in 0..4 {
            let start = 16 + idx * 8;
            buf[start..start + 8].copy_from_slice(&self.linguistics[idx].to_le_bytes());
        }
        for idx in 0..4 {
            let start = 48 + idx * 8;
            buf[start..start + 8].copy_from_slice(&self.tech[idx].to_le_bytes());
        }
        buf[80..84].copy_from_slice(&self.population.to_le_bytes());
        buf
    }

    pub fn as_bytes(&self) -> [u8; Self::BYTE_SIZE] {
        let mut buf = [0u8; Self::BYTE_SIZE];
        buf[..Self::BODY_SIZE].copy_from_slice(&self.body_bytes());
        buf[Self::SIGNATURE_OFFSET..].copy_from_slice(&self.signature);
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != Self::BYTE_SIZE {
            return None;
        }

        let mut linguistics = [0u64; 4];
        for (idx, slot) in linguistics.iter_mut().enumerate() {
            let start = 16 + idx * 8;
            *slot = u64::from_le_bytes(bytes[start..start + 8].try_into().ok()?);
        }

        let mut tech = [0u64; 4];
        for (idx, slot) in tech.iter_mut().enumerate() {
            let start = 48 + idx * 8;
            *slot = u64::from_le_bytes(bytes[start..start + 8].try_into().ok()?);
        }

        Some(Self {
            taxonomy: u64::from_le_bytes(bytes[0..8].try_into().ok()?),
            brain: u64::from_le_bytes(bytes[8..16].try_into().ok()?),
            linguistics,
            tech,
            population: u32::from_le_bytes(bytes[80..84].try_into().ok()?),
            signature: bytes[Self::SIGNATURE_OFFSET..Self::BYTE_SIZE]
                .try_into()
                .ok()?,
        })
    }
}

#[derive(Resource, Default)]
pub struct WormholeActivity(pub u8);

#[derive(Resource, Default)]
pub struct WormholeRuntime {
    pub rx: Option<UnixDatagram>,
    pub tx: Option<UnixDatagram>,
    pub tx_path: Option<PathBuf>,
    pub rx_path: Option<PathBuf>,
}

pub fn ascension_system(
    mut commands: Commands,
    runtime: Res<WormholeRuntime>,
    signing_key: Res<SigningKeyResource>,
    mut activity: ResMut<WormholeActivity>,
    query: Query<(
        Entity,
        &Action,
        &Taxonomy,
        &SimHashBrain,
        &LinguisticSequence,
        &TechnologyMask,
        &Population,
    )>,
) {
    let (Some(socket), Some(tx_path)) = (runtime.tx.as_ref(), runtime.tx_path.as_ref()) else {
        return;
    };

    for (entity, action, taxonomy, brain, linguistics, tech, population) in query.iter() {
        if *action != Action::Ascend {
            continue;
        }

        let mut payload = WormholePayload {
            taxonomy: taxonomy.0,
            brain: brain.0,
            linguistics: linguistics.0,
            tech: tech.0,
            population: population.0,
            signature: [0u8; 64],
        };
        payload.signature = signing_key.0.sign(&payload.body_bytes()).to_bytes();

        if socket.send_to(&payload.as_bytes(), tx_path).is_ok() {
            activity.0 |= WORMHOLE_ACTIVITY_OUTGOING;
            commands.entity(entity).despawn();
        }
    }
}

pub fn arrival_system(
    mut commands: Commands,
    runtime: Res<WormholeRuntime>,
    signing_key: Res<SigningKeyResource>,
    mut activity: ResMut<WormholeActivity>,
    query: Query<&CivIndex>,
) {
    let Some(socket) = runtime.rx.as_ref() else {
        return;
    };

    let verifying_key = signing_key.0.verifying_key();
    let mut next_id = query.iter().map(|c| c.0).max().unwrap_or(0) + 1;
    let mut buf = [0u8; WormholePayload::BYTE_SIZE];

    loop {
        match socket.recv(&mut buf) {
            Ok(size) if size == WormholePayload::BYTE_SIZE => {
                let Some(payload) = WormholePayload::from_bytes(&buf) else {
                    continue;
                };
                let signature = Signature::from_bytes(&payload.signature);
                if verifying_key
                    .verify(&buf[..WormholePayload::BODY_SIZE], &signature)
                    .is_err()
                {
                    continue;
                }

                let x = (payload.taxonomy as u8) % 64;
                let y = (payload.brain as u8) % 16;
                commands.spawn((
                    Population(payload.population.max(100)),
                    Taxonomy(payload.taxonomy),
                    LinguisticSequence(payload.linguistics),
                    SimHashBrain(payload.brain),
                    Position { x, y },
                    Action::Idle,
                    CivIndex(next_id),
                    TechnologyMask(payload.tech),
                    MiracleGrace(100),
                    NewlySpawned,
                ));
                next_id += 1;
                activity.0 |= WORMHOLE_ACTIVITY_INCOMING;
            }
            Ok(_) => continue,
            Err(err) if err.kind() == ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }
}
