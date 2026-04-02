//! Miracle IPC Substrate for the Laplace Oracle.
#![forbid(unsafe_code)]
#![allow(clippy::suspicious_open_options)]

use crate::biology::Taxonomy;
use crate::intelligence::{
    linguistic_sequence_from_taxonomy, LinguisticSequence, NewlySpawned, SimHashBrain,
    TechnologyMask,
};
use crate::physics::EnvironmentStack;
use crate::temporal::{Population, Position};
use bevy_ecs::prelude::*;
use memmap2::MmapMut;
use std::io::{ErrorKind, Read};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

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
    Pause = 5,
    Play = 6,
    Speed = 7,
    Flood = 8,
    Drought = 9,
}

impl From<u8> for MiracleType {
    fn from(v: u8) -> Self {
        match v {
            1 => Self::Fire,
            2 => Self::Rain,
            3 => Self::Build,
            4 => Self::Infect,
            5 => Self::Pause,
            6 => Self::Play,
            7 => Self::Speed,
            8 => Self::Flood,
            9 => Self::Drought,
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
        Self {
            nonce,
            miracle_type,
            target_x,
            target_y,
            radius,
            payload,
        }
    }
}

// ── Resources ────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct LastMiracleNonce(pub u64);

#[derive(Resource)]
pub struct MiracleMmap(pub MmapMut);

#[derive(Resource)]
pub struct ApiListenerRuntime {
    pub listener: UnixListener,
    pub path: PathBuf,
}

#[derive(Resource)]
pub struct TemporalState {
    pub paused: bool,
    pub speed_ms: u64,
}

impl Default for TemporalState {
    fn default() -> Self {
        Self {
            paused: false,
            speed_ms: 16,
        }
    }
}

#[derive(Component)]
pub struct MiracleGrace(pub u32);

pub fn bind_api_listener(path: &Path) -> std::io::Result<UnixListener> {
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    let listener = UnixListener::bind(path)?;
    listener.set_nonblocking(true)?;
    Ok(listener)
}

pub fn api_listener_system(
    mut mmap: ResMut<MiracleMmap>,
    runtime: Option<Res<ApiListenerRuntime>>,
) {
    let Some(runtime) = runtime else {
        return;
    };

    loop {
        match runtime.listener.accept() {
            Ok((mut stream, _addr)) => {
                let _ = stream.set_nonblocking(true);
                let mut bytes = [0u8; 20];
                match stream.read(&mut bytes) {
                    Ok(20) => {
                        let cmd = MiracleCommand::from_bytes(&bytes);
                        mmap.0[0..20].copy_from_slice(&cmd.to_bytes());
                        let _ = mmap.0.flush();
                    }
                    Ok(_) => continue,
                    Err(err) if err.kind() == ErrorKind::WouldBlock => continue,
                    Err(_) => continue,
                }
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }
}

// ── Systems ──────────────────────────────────────────────────────────────────

pub fn genesis_listener_system(
    mmap: Res<MiracleMmap>,
    mut last_nonce: ResMut<LastMiracleNonce>,
    mut env: ResMut<EnvironmentStack>,
    mut temp_state: ResMut<TemporalState>,
    query: Query<&crate::temporal::CivIndex>,
    mut commands: Commands,
) {
    let bytes = &mmap.0[0..20];
    let cmd = MiracleCommand::from_bytes(bytes);

    if cmd.nonce > last_nonce.0 {
        last_nonce.0 = cmd.nonce;
        let m_type = MiracleType::from(cmd.miracle_type);

        match m_type {
            MiracleType::Genesis => {
                let max_index = query.iter().map(|c| c.0).max().unwrap_or(0);
                let next_id = max_index + 1;
                let pos = Position {
                    x: cmd.target_x % 64,
                    y: cmd.target_y % 16,
                };
                let taxonomy = Taxonomy(cmd.payload);
                let linguistic_sequence = linguistic_sequence_from_taxonomy(taxonomy);
                for _ in 0..100 {
                    commands.spawn((
                        Population(100),
                        taxonomy,
                        LinguisticSequence(linguistic_sequence.0),
                        SimHashBrain(HUMANOID),
                        pos,
                        crate::intelligence::Action::Idle,
                        crate::temporal::CivIndex(next_id),
                        TechnologyMask::default(),
                        MiracleGrace(100),
                        NewlySpawned,
                    ));
                }
                env.biomass[pos.y as usize] |= 1 << (pos.x as usize);
                temp_state.paused = false;
            }
            MiracleType::Fire => {
                let tx = cmd.target_x as i16;
                let ty = cmd.target_y as i16;
                let r = cmd.radius as i16;
                for dy in -r..=r {
                    for dx in -r..=r {
                        let x = (tx + dx).rem_euclid(64) as usize;
                        let y = (ty + dy).rem_euclid(16) as usize;
                        let bit = 1 << x;
                        env.temperature[y] |= bit;
                        env.biomass[y] &= !bit; // Fire consumes biomass
                    }
                }
            }
            MiracleType::Rain => {
                let tx = cmd.target_x as i16;
                let ty = cmd.target_y as i16;
                let r = cmd.radius as i16;
                for dy in -r..=r {
                    for dx in -r..=r {
                        let x = (tx + dx).rem_euclid(64) as usize;
                        let y = (ty + dy).rem_euclid(16) as usize;
                        let bit = 1 << x;
                        env.water[y] |= bit;
                        env.temperature[y] &= !bit; // Rain cools fire
                    }
                }
            }
            MiracleType::Build => {
                let tx = cmd.target_x as i16;
                let ty = cmd.target_y as i16;
                let r = cmd.radius as i16;
                for dy in -r..=r {
                    for dx in -r..=r {
                        let x = (tx + dx).rem_euclid(64) as usize;
                        let y = (ty + dy).rem_euclid(16) as usize;
                        let bit = 1 << x;
                        env.structure[y] |= bit;
                        env.biomass[y] &= !bit; // Build consumes biomass
                    }
                }
            }
            MiracleType::Infect => {
                let tx = cmd.target_x as usize;
                let ty = cmd.target_y as usize;
                if tx < 64 && ty < 16 {
                    env.memetics[ty * 64 + tx] = cmd.payload;
                }
            }
            MiracleType::Pause => temp_state.paused = true,
            MiracleType::Play => temp_state.paused = false,
            MiracleType::Speed => temp_state.speed_ms = cmd.payload.clamp(1, 1000),
            MiracleType::Flood => {
                let tx = cmd.target_x as i16;
                let ty = cmd.target_y as i16;
                let r = cmd.radius as i16;
                for dy in -r..=r {
                    for dx in -r..=r {
                        let x = (tx + dx).rem_euclid(64) as usize;
                        let y = (ty + dy).rem_euclid(16) as usize;
                        env.water[y] |= 1 << x;
                    }
                }
            }
            MiracleType::Drought => {
                let tx = cmd.target_x as i16;
                let ty = cmd.target_y as i16;
                let r = cmd.radius as i16;
                for dy in -r..=r {
                    for dx in -r..=r {
                        let x = (tx + dx).rem_euclid(64) as usize;
                        let y = (ty + dy).rem_euclid(16) as usize;
                        env.water[y] &= !(1 << x);
                    }
                }
            }
        }
    }
}
