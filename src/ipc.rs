//! Miracle IPC Substrate for the Laplace Oracle.
#![forbid(unsafe_code)]

use crate::biology::Taxonomy;
use bevy_ecs::prelude::*;
use crate::physics::EnvironmentStack;
use crate::intelligence::{SimHashBrain, TechnologyMask, NewlySpawned};
use crate::temporal::{Population, Position};
use memmap2::MmapMut;

// ── Species Masks ────────────────────────────────────────────────────────────

pub const MICROBE: u64 = 0x000F;
pub const INSECTOID: u64 = 0x0F0F;
pub const HUMANOID: u64 = 0x7FFF;

pub fn open_miracle_file() -> std::fs::File {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("miracles.db")
        .expect("open miracles.db")
}

// ── IPC Structures ───────────────────────────────────────────────────────────

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiracleType {
    Genesis = 0,
    Fire = 1,
    Rain = 2,
    Build = 3,
    Infect = 4,
}

impl From<u8> for MiracleType {
    fn from(v: u8) -> Self {
        match v {
            1 => Self::Fire,
            2 => Self::Rain,
            3 => Self::Build,
            4 => Self::Infect,
            _ => Self::Genesis,
        }
    }
}

#[repr(C, packed)]
pub struct MiracleCommand {
    pub nonce: u64,
    pub miracle_type: u8,
    pub target_x: u8,
    pub target_y: u8,
    pub radius: u8,
    pub payload: u64,
}

impl MiracleCommand {
    pub fn to_bytes(&self) -> [u8; 20] {
        let mut bytes = [0u8; 20];
        bytes[0..8].copy_from_slice(&self.nonce.to_le_bytes());
        bytes[8] = self.miracle_type;
        bytes[9] = self.target_x;
        bytes[10] = self.target_y;
        bytes[11] = self.radius;
        bytes[12..20].copy_from_slice(&self.payload.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let nonce = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let miracle_type = bytes[8];
        let target_x = bytes[9];
        let target_y = bytes[10];
        let radius = bytes[11];
        let payload = u64::from_le_bytes(bytes[12..20].try_into().unwrap());
        Self { nonce, miracle_type, target_x, target_y, radius, payload }
    }
}

// ── Resources ────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct LastMiracleNonce(pub u64);

#[derive(Resource)]
pub struct MiracleMmap(pub MmapMut);

#[derive(Component)]
pub struct MiracleGrace(pub u32);

// ── Systems ──────────────────────────────────────────────────────────────────

pub fn genesis_listener_system(
    mmap: Res<MiracleMmap>,
    mut last_nonce: ResMut<LastMiracleNonce>,
    mut env: ResMut<EnvironmentStack>,
    query: Query<&crate::temporal::CivIndex>,
    mut commands: Commands,
) {
    let bytes = &mmap.0[0..20];
    let cmd = MiracleCommand::from_bytes(bytes);

    if cmd.nonce > last_nonce.0 {
        // 1. ATOMIC TRANSACTION: Consume nonce first
        last_nonce.0 = cmd.nonce;

        if cmd.miracle_type == MiracleType::Genesis as u8 {
            // 2. State Lookup
            let max_index = query.iter().map(|c| c.0).max().unwrap_or(0);
            let next_id = max_index + 1;

            let pos = Position { x: cmd.target_x % 64, y: cmd.target_y % 16 };

            // 3. Spawning (transactional batch)
            for _ in 0..100 {
                commands.spawn((
                    Population(100),
                    Taxonomy(cmd.payload), // payload is species mask for Genesis
                    SimHashBrain(HUMANOID), // Hardcoded cognitive mask
                    pos,
                    crate::intelligence::Action::Idle,
                    crate::temporal::CivIndex(next_id),
                    TechnologyMask::default(),
                    MiracleGrace(100), // Protect for 100 ticks
                    NewlySpawned,
                ));
            }

            // 4. EnvironmentStack biomass seeding
            let row = pos.y as usize;
            let col = pos.x as usize;
            env.biomass[row] |= 1 << col;
        } else if cmd.miracle_type == MiracleType::Fire as u8 {
            // Ignition logic: Set Temperature layer in radius
            let tx = cmd.target_x as i16;
            let ty = cmd.target_y as i16;
            let r = cmd.radius as i16;
            for dy in -r..=r {
                for dx in -r..=r {
                    let x = tx + dx;
                    let y = ty + dy;
                    if x >= 0 && x < 64 && y >= 0 && y < 16 {
                        env.temperature[y as usize] |= 1 << (x as usize);
                    }
                }
            }
        }
    }
}
