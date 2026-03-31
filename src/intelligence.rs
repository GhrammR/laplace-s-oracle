//! LSH Cognitive Engine for the Laplace Oracle.
#![allow(unknown_lints)]
#![deny(clippy::alloc_id)]
#![forbid(unsafe_code)]

use bevy_ecs::prelude::*;
use rand::Rng;
use crate::temporal::{Position};
use crate::physics::EnvironmentStack;

// ── Components & Structs ──────────────────────────────────────────────────────

/// Component representing the entity's cognitive signature for LSH.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimHashBrain(pub u64);

/// Environmental stimulus for Hamming distance calculation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stimulus(pub u64);

/// 256-bit Technology Mask.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TechnologyMask(pub [u64; 4]);

impl TechnologyMask {
    pub fn bit_count(&self) -> u32 {
        self.0.iter().map(|&x| x.count_ones()).sum()
    }

    pub fn is_bit_set(&self, bit: usize) -> bool {
        if bit >= 256 { return false; }
        let word = bit / 64;
        let pos = bit % 64;
        (self.0[word] >> pos) & 1 == 1
    }

    pub fn set_bit(&mut self, bit: usize) {
        if bit >= 256 { return; }
        let word = bit / 64;
        let pos = bit % 64;
        self.0[word] |= 1 << pos;
    }
}

pub fn spatial_conflict_system(
    mut commands: Commands,
    query: Query<(Entity, &Position, &SimHashBrain)>,
) {
    let mut entities: Vec<_> = query.iter().collect();

    // O(N log N) sort by position
    entities.sort_by_key(|&(_, pos, _)| (pos.x, pos.y));

    let mut i = 0;
    while i < entities.len() {
        let (current_entity, current_pos, current_brain) = entities[i];
        let mut conflicts = vec![(current_entity, current_brain.0.count_ones())];
        let mut j = i + 1;

        while j < entities.len() {
            let (next_entity, next_pos, next_brain) = entities[j];
            if next_pos.x == current_pos.x && next_pos.y == current_pos.y {
                conflicts.push((next_entity, next_brain.0.count_ones()));
                j += 1;
            } else {
                break;
            }
        }

        if conflicts.len() > 1 {
            // A conflict occurred
            conflicts.sort_by_key(|&(_, fitness)| std::cmp::Reverse(fitness));

            // The first one is the winner, despawn the rest
            for (loser_entity, _) in conflicts.iter().skip(1) {
                commands.entity(*loser_entity).despawn();
            }
        }

        i = j;
    }
}

/// Procedural Technology Discovery logic.
/// Selects a bit to flip based on candidacy (neighboring bits) and world_hash.
pub fn discover_tech(mask: &mut TechnologyMask, world_hash: &[u8; 32]) -> Option<u64> {
    let mut candidates = Vec::with_capacity(256);

    for i in 0..256 {
        if !mask.is_bit_set(i) {
            // Candidacy: Index 0 is always a candidate if not set.
            // Other bits are candidates if a neighbor is set.
            let mut is_candidate = i == 0;
            if !is_candidate && i > 0 && mask.is_bit_set(i - 1) {
                is_candidate = true;
            }
            if !is_candidate && i < 255 && mask.is_bit_set(i + 1) {
                is_candidate = true;
            }

            if is_candidate {
                candidates.push(i);
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }

    // Select index deterministically using world_hash
    // Use the first 8 bytes of world_hash as a seed
    let mut seed = 0u64;
    for i in 0..8 {
        seed = (seed << 8) | (world_hash[i] as u64);
    }

    let selection = (seed as usize) % candidates.len();
    let bit_index = candidates[selection];
    mask.set_bit(bit_index);

    // --- PHASE VII: Meme Generation ---
    // Generate a 64-bit meme hash from the discovered tech bit and world hash
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&bit_index.to_le_bytes());
    hasher.update(world_hash);
    let result = hasher.finalize();
    let mut meme_hash = 0u64;
    for i in 0..8 {
        meme_hash = (meme_hash << 8) | (result[i] as u64);
    }
    Some(meme_hash)
}

/// Discrete actions mapped from Hamming distance.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Action {
    #[default]
    Idle,
    Expand,
    Research,
    Defend,
    Build,
    Flee,
}

pub const FEAR_MASK: u64 = 0x8000_0000_0000_0000;
pub const WARFARE_COGNITIVE_MASK: u64 = 0xF0F0_F0F0_0F0F_0F0F;

// ── Decision Engine ───────────────────────────────────────────────────────────

pub trait DecisionEngine {
    fn decide(brain: &SimHashBrain, stimulus: &Stimulus, research_threshold: u32) -> Action;
}

pub struct Intelligence;

impl DecisionEngine for Intelligence {
    fn decide(brain: &SimHashBrain, stimulus: &Stimulus, research_threshold: u32) -> Action {
        let distance = (brain.0 ^ stimulus.0).count_ones();

        // Hamming distance DFA mapping with Malthusian scaling
        if distance <= research_threshold {
            Action::Research
        } else if distance <= 32 {
            Action::Expand
        } else if distance <= 48 {
            Action::Defend
        } else {
            Action::Idle
        }
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Global planetary state summary.
#[derive(Resource, Default)]
pub struct EnvironmentData(pub u64);

/// Marker component for newly spawned entities, to trigger mutation.
#[derive(Component)]
pub struct NewlySpawned;

/// System that applies genetic mutation to newly spawned entities.
pub fn mutation_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut SimHashBrain), With<NewlySpawned>>,
    mut rng_res: ResMut<crate::temporal::RngResource>,
) {
    for (entity, mut brain) in query.iter_mut() {
        if rng_res.rng.gen_bool(0.001) {
            let bit_to_flip = rng_res.rng.gen_range(0..64);
            brain.0 ^= 1 << bit_to_flip;
        }
        commands.entity(entity).remove::<NewlySpawned>();
    }
}

/// System that updates Action and TechnologyMask based on the global EnvironmentData and Population Density.
pub fn think_system(
    env_data: Res<EnvironmentData>,
    mut env_stack: ResMut<EnvironmentStack>,
    world_hash: Res<crate::telemetry::WorldHash>,
    mut query: Query<(&mut SimHashBrain, &mut Action, &mut TechnologyMask, &Position)>,
) {
    let stimulus = Stimulus(env_data.0);

    const RESEARCH_THRESHOLD: u32 = 16;
    const METALLURGY_BIT: usize = 64;

    for (mut brain, mut action, mut tech, pos) in query.iter_mut() {
        let x = pos.x as usize;
        let y = pos.y as usize;
        let meme_idx = y * 64 + x;

        // --- PHASE VII: Cognitive Infection ---
        let current_meme = env_stack.memetics[meme_idx];
        if current_meme != 0 {
            // Check for specific memes, e.g. "Warfare" meme tied to bit 64 (Metallurgy)
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&METALLURGY_BIT.to_le_bytes());
            hasher.update(&world_hash.0);
            let result = hasher.finalize();
            let mut warfare_hash = 0u64;
            for i in 0..8 {
                warfare_hash = (warfare_hash << 8) | (result[i] as u64);
            }

            if current_meme == warfare_hash {
                brain.0 ^= WARFARE_COGNITIVE_MASK; // Infectious aggression
            }
        }

        // --- Cognitive Reaction: Fire Detection ---
        let mut fire_nearby = false;
        let range = 1i16;
        for dy in -range..=range {
            let ny = (pos.y as i16 + dy).rem_euclid(16) as usize;
            let row_t = env_stack.temperature[ny];
            for dx in -range..=range {
                let nx = (pos.x as i16 + dx).rem_euclid(64) as usize;
                if (row_t >> nx) & 1 == 1 {
                    fire_nearby = true;
                    break;
                }
            }
            if fire_nearby { break; }
        }

        if fire_nearby {
            brain.0 ^= FEAR_MASK;
            *action = Action::Flee;
        } else {
            // Normal Decision
            *action = Intelligence::decide(&brain, &stimulus, RESEARCH_THRESHOLD);

            // Procedural Technology Discovery
            if matches!(*action, Action::Research) {
                if let Some(new_meme) = discover_tech(&mut tech, &world_hash.0) {
                    env_stack.memetics[meme_idx] = new_meme;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::EnvironmentStack;
    use crate::telemetry::WorldHash;

    #[test]
    fn test_cognitive_modification() {
        let mut world = World::new();
        world.insert_resource(EnvironmentData::default());
        world.insert_resource(EnvironmentStack::default());
        world.insert_resource(WorldHash([42u8; 32]));

        // Spawn entity at (5, 5)
        let entity = world.spawn((
            SimHashBrain(0),
            Action::Idle,
            TechnologyMask::default(),
            Position { x: 5, y: 5 },
        )).id();

        // Metallurgy bit 64 hash
        const METALLURGY_BIT: usize = 64;
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&METALLURGY_BIT.to_le_bytes());
        hasher.update(&[42u8; 32]);
        let result = hasher.finalize();
        let mut warfare_hash = 0u64;
        for i in 0..8 {
            warfare_hash = (warfare_hash << 8) | (result[i] as u64);
        }

        // Set the "Warfare" meme hash at (5, 5)
        {
            let mut env = world.resource_mut::<EnvironmentStack>();
            env.memetics[5 * 64 + 5] = warfare_hash;
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(think_system);
        schedule.run(&mut world);

        let brain = world.get::<SimHashBrain>(entity).unwrap();
        assert_eq!(brain.0, WARFARE_COGNITIVE_MASK);
    }
}

/// System that executes the physical effects of actions.
pub fn action_processing_system(
    mut env: ResMut<EnvironmentStack>,
    mut query: Query<(&Action, &mut Position)>,
    mut rng: ResMut<crate::temporal::RngResource>,
) {
    for (action, mut pos) in query.iter_mut() {
        match action {
            Action::Flee => {
                // Random move to a non-fire coordinate in 3x3 neighborhood
                let mut candidates = Vec::with_capacity(9);
                for dy in -1..=1 {
                    let ny = (pos.y as i16 + dy).rem_euclid(16) as usize;
                    let row_t = env.temperature[ny];
                    for dx in -1..=1 {
                        let nx = (pos.x as i16 + dx).rem_euclid(64) as usize;
                        if (row_t >> nx) & 1 == 0 {
                            candidates.push((nx as u8, ny as u8));
                        }
                    }
                }
                if !candidates.is_empty() {
                    let idx = rng.rng.gen_range(0..candidates.len());
                    pos.x = candidates[idx].0;
                    pos.y = candidates[idx].1;
                }
            }
            Action::Build => {
                let x = pos.x as usize;
                let y = pos.y as usize;
                if (env.biomass[y] >> x) & 1 == 1 {
                    env.biomass[y] &= !(1 << x); // Consume biomass
                    env.structure[y] |= 1 << x;  // Set structure
                }
            }
            _ => {}
        }
    }
}
