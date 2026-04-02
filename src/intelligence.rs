//! LSH Cognitive Engine for the Laplace Oracle.
#![allow(unknown_lints)]
#![deny(clippy::alloc_id)]
#![forbid(unsafe_code)]

use crate::biology::Taxonomy;
use crate::physics::{local_memetics_signature, EnvironmentStack};
use crate::temporal::{Population, Position};
use bevy_ecs::prelude::*;
use rand::Rng;
use sha2::{Digest, Sha256};

/// Component representing the entity's cognitive signature for LSH.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimHashBrain(pub u64);

/// 256-bit linguistic state carried by an entity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LinguisticSequence(pub [u64; 4]);

impl LinguisticSequence {
    pub fn first_byte(self) -> u8 {
        (self.0[0] & 0xFF) as u8
    }
}

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
        if bit >= 256 {
            return false;
        }
        let word = bit / 64;
        let pos = bit % 64;
        (self.0[word] >> pos) & 1 == 1
    }

    pub fn set_bit(&mut self, bit: usize) {
        if bit >= 256 {
            return;
        }
        let word = bit / 64;
        let pos = bit % 64;
        self.0[word] |= 1 << pos;
    }
}

pub fn linguistic_sequence_from_taxonomy(taxonomy: Taxonomy) -> LinguisticSequence {
    let digest = Sha256::digest(taxonomy.0.to_le_bytes());
    let mut words = [0u64; 4];
    for (i, chunk) in digest.chunks_exact(8).enumerate() {
        words[i] = u64::from_le_bytes(chunk.try_into().unwrap());
    }
    LinguisticSequence(words)
}

pub fn cultural_alignment(sequence: &LinguisticSequence, local_memetics_bit: &[u64; 4]) -> u32 {
    256 - sequence
        .0
        .iter()
        .zip(local_memetics_bit.iter())
        .map(|(lhs, rhs)| (lhs ^ rhs).count_ones())
        .sum::<u32>()
}

fn interleaved_crossover(
    lhs: &LinguisticSequence,
    rhs: &LinguisticSequence,
) -> (LinguisticSequence, LinguisticSequence) {
    let even_mask = 0x5555_5555_5555_5555u64;
    let odd_mask = !even_mask;
    let mut next_lhs = [0u64; 4];
    let mut next_rhs = [0u64; 4];

    for idx in 0..4 {
        next_lhs[idx] = (lhs.0[idx] & even_mask) | (rhs.0[idx] & odd_mask);
        next_rhs[idx] = (rhs.0[idx] & even_mask) | (lhs.0[idx] & odd_mask);
    }

    (LinguisticSequence(next_lhs), LinguisticSequence(next_rhs))
}

fn apply_innovation(sequence: &mut LinguisticSequence, rng: &mut impl Rng) {
    let bit = rng.gen_range(0..256);
    let word = bit / 64;
    let offset = bit % 64;
    sequence.0[word] ^= 1u64 << offset;
}

pub fn linguistic_trade_system(
    mut query: Query<(&Position, &mut LinguisticSequence)>,
    mut rng_res: ResMut<crate::temporal::RngResource>,
) {
    let mut pairs = query.iter_combinations_mut();
    while let Some([(pos_a, mut seq_a), (pos_b, mut seq_b)]) = pairs.fetch_next() {
        if pos_a.x != pos_b.x || pos_a.y != pos_b.y {
            continue;
        }

        let (next_a, next_b) = interleaved_crossover(&seq_a, &seq_b);
        *seq_a = next_a;
        *seq_b = next_b;

        if rng_res.rng.gen_ratio(1, 32) {
            if rng_res.rng.gen_bool(0.5) {
                apply_innovation(&mut seq_a, &mut rng_res.rng);
            } else {
                apply_innovation(&mut seq_b, &mut rng_res.rng);
            }
        }
    }
}

pub fn spatial_conflict_system(
    mut commands: Commands,
    env: Res<EnvironmentStack>,
    query: Query<(Entity, &Position, &SimHashBrain, &LinguisticSequence)>,
) {
    let mut entities: Vec<_> = query.iter().collect();

    entities.sort_by_key(|&(_, pos, _, _)| (pos.x, pos.y));

    let mut i = 0;
    while i < entities.len() {
        let (current_entity, current_pos, current_brain, current_sequence) = entities[i];
        let local_memetics = local_memetics_signature(&env, current_pos);
        let mut conflicts = vec![(
            current_entity,
            current_brain.0.count_ones() + cultural_alignment(current_sequence, &local_memetics),
        )];
        let mut j = i + 1;

        while j < entities.len() {
            let (next_entity, next_pos, next_brain, next_sequence) = entities[j];
            if next_pos.x == current_pos.x && next_pos.y == current_pos.y {
                conflicts.push((
                    next_entity,
                    next_brain.0.count_ones() + cultural_alignment(next_sequence, &local_memetics),
                ));
                j += 1;
            } else {
                break;
            }
        }

        if conflicts.len() > 1 {
            conflicts.sort_by_key(|&(_, fitness)| std::cmp::Reverse(fitness));
            for (loser_entity, _) in conflicts.iter().skip(1) {
                commands.entity(*loser_entity).despawn();
            }
        }

        i = j;
    }
}

pub fn discover_tech(mask: &mut TechnologyMask, world_hash: &[u8; 32]) -> Option<u64> {
    let mut candidates = Vec::with_capacity(256);

    for i in 0..256 {
        if !mask.is_bit_set(i) {
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

    let mut seed = 0u64;
    for byte in world_hash.iter().take(8) {
        seed = (seed << 8) | (*byte as u64);
    }

    let selection = (seed as usize) % candidates.len();
    let bit_index = candidates[selection];
    mask.set_bit(bit_index);

    let mut hasher = Sha256::new();
    hasher.update(bit_index.to_le_bytes());
    hasher.update(world_hash);
    let result = hasher.finalize();
    let mut meme_hash = 0u64;
    for byte in result.iter().take(8) {
        meme_hash = (meme_hash << 8) | (*byte as u64);
    }
    Some(meme_hash)
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Action {
    #[default]
    Idle,
    Expand,
    Research,
    Defend,
    Build,
    Flee,
    Ascend,
}

pub const FEAR_MASK: u64 = 0x8000_0000_0000_0000;
pub const WARFARE_COGNITIVE_MASK: u64 = 0xF0F0_F0F0_0F0F_0F0F;
pub const SEMICONDUCTOR_BIT: usize = 128;
pub const ASCENSION_BIT: usize = crate::wormhole::WORMHOLE_ASCENSION_BIT;

pub trait DecisionEngine {
    fn decide(brain: &SimHashBrain, stimulus: &Stimulus, research_threshold: u32) -> Action;
}

pub struct Intelligence;

impl DecisionEngine for Intelligence {
    fn decide(brain: &SimHashBrain, stimulus: &Stimulus, research_threshold: u32) -> Action {
        let distance = (brain.0 ^ stimulus.0).count_ones();

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

#[derive(Resource, Default)]
pub struct EnvironmentData(pub u64);

#[derive(Component)]
pub struct NewlySpawned;

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

pub fn think_system(
    env_data: Res<EnvironmentData>,
    mut env_stack: ResMut<EnvironmentStack>,
    world_hash: Res<crate::telemetry::WorldHash>,
    mut query: Query<(
        &mut SimHashBrain,
        &mut Action,
        &mut TechnologyMask,
        &Position,
        &Population,
    )>,
) {
    let stimulus = Stimulus(env_data.0);

    const RESEARCH_THRESHOLD: u32 = 16;
    const METALLURGY_BIT: usize = 64;

    for (mut brain, mut action, mut tech, pos, population) in query.iter_mut() {
        let x = pos.x as usize;
        let y = pos.y as usize;
        let meme_idx = y * 64 + x;

        let current_meme = env_stack.memetics[meme_idx];
        if current_meme != 0 {
            let mut hasher = Sha256::new();
            hasher.update(METALLURGY_BIT.to_le_bytes());
            hasher.update(world_hash.0);
            let result = hasher.finalize();
            let mut warfare_hash = 0u64;
            for byte in result.iter().take(8) {
                warfare_hash = (warfare_hash << 8) | (*byte as u64);
            }

            if current_meme == warfare_hash {
                brain.0 ^= WARFARE_COGNITIVE_MASK;
            }
        }

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
            if fire_nearby {
                break;
            }
        }

        if fire_nearby {
            brain.0 ^= FEAR_MASK;
            *action = Action::Flee;
        } else {
            let local_pressure = ((env_stack.pressure[y] >> x) & 1) == 1;
            let critical_density = population.0 >= 1_500;
            if tech.is_bit_set(ASCENSION_BIT) && critical_density && local_pressure {
                *action = Action::Ascend;
                continue;
            }

            *action = Intelligence::decide(&brain, &stimulus, RESEARCH_THRESHOLD);

            if matches!(*action, Action::Research) {
                if let Some(new_meme) = discover_tech(&mut tech, &world_hash.0) {
                    env_stack.memetics[meme_idx] = new_meme;
                }
            }

            if tech.is_bit_set(SEMICONDUCTOR_BIT)
                && matches!(*action, Action::Research | Action::Build)
            {
                let logic_bit = 1u64 << x;
                if (env_stack.biomass[y] & logic_bit) != 0 {
                    env_stack.logic[y] ^= logic_bit;
                    env_stack.biomass[y] &= !logic_bit;
                }
            }
        }
    }
}

pub fn action_processing_system(
    mut env: ResMut<EnvironmentStack>,
    mut query: Query<(&Action, &mut Position)>,
    mut rng: ResMut<crate::temporal::RngResource>,
) {
    for (action, mut pos) in query.iter_mut() {
        match action {
            Action::Flee => {
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
                    env.biomass[y] &= !(1 << x);
                    env.structure[y] |= 1 << x;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::EnvironmentStack;
    use crate::telemetry::WorldHash;
    use bevy_ecs::system::RunSystemOnce;

    #[test]
    fn test_cognitive_modification() {
        let mut world = World::new();
        world.insert_resource(EnvironmentData::default());
        world.insert_resource(EnvironmentStack::default());
        world.insert_resource(WorldHash([42u8; 32]));

        let entity = world
            .spawn((
                SimHashBrain(0),
                Action::Idle,
                TechnologyMask::default(),
                Position { x: 5, y: 5 },
                Population(100),
            ))
            .id();

        const METALLURGY_BIT: usize = 64;
        let mut hasher = Sha256::new();
        hasher.update(METALLURGY_BIT.to_le_bytes());
        hasher.update([42u8; 32]);
        let result = hasher.finalize();
        let mut warfare_hash = 0u64;
        for byte in result.iter().take(8) {
            warfare_hash = (warfare_hash << 8) | (*byte as u64);
        }

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

    #[test]
    fn test_linguistic_sequence_is_deterministic() {
        let taxonomy = Taxonomy(0x1234_5678_90AB_CDEF);
        assert_eq!(
            linguistic_sequence_from_taxonomy(taxonomy),
            linguistic_sequence_from_taxonomy(taxonomy)
        );
    }

    #[test]
    fn test_semiconductor_clock_pulse() {
        let mut world = World::new();
        world.insert_resource(EnvironmentData::default());
        world.insert_resource(EnvironmentStack::default());
        world.insert_resource(WorldHash([7u8; 32]));

        {
            let mut env = world.resource_mut::<EnvironmentStack>();
            env.biomass[4] |= 1 << 3;
        }

        let mut mask = TechnologyMask::default();
        mask.set_bit(SEMICONDUCTOR_BIT);

        world.spawn((
            SimHashBrain(0),
            Action::Build,
            mask,
            Position { x: 3, y: 4 },
            Population(100),
        ));

        let mut schedule = Schedule::default();
        schedule.add_systems(think_system);
        schedule.run(&mut world);

        let env = world.resource::<EnvironmentStack>();
        assert_eq!((env.logic[4] >> 3) & 1, 1);
        assert_eq!((env.biomass[4] >> 3) & 1, 0);
    }

    #[test]
    fn test_linguistic_trade_interleaves_bits() {
        let lhs = LinguisticSequence([0xAAAA_AAAA_AAAA_AAAA; 4]);
        let rhs = LinguisticSequence([0x5555_5555_5555_5555; 4]);
        let (next_lhs, next_rhs) = interleaved_crossover(&lhs, &rhs);
        assert_eq!(next_lhs.0, [0; 4]);
        assert_eq!(next_rhs.0, [u64::MAX; 4]);
    }

    #[test]
    fn test_ascension_gate_chooses_ascend() {
        let mut world = World::new();
        world.insert_resource(EnvironmentData::default());
        world.insert_resource(EnvironmentStack::default());
        world.insert_resource(WorldHash([9u8; 32]));

        {
            let mut env = world.resource_mut::<EnvironmentStack>();
            env.pressure[6] |= 1 << 5;
        }

        let mut mask = TechnologyMask::default();
        mask.set_bit(ASCENSION_BIT);

        let entity = world
            .spawn((
                SimHashBrain(0),
                Action::Idle,
                mask,
                Position { x: 5, y: 6 },
                Population(1_500),
            ))
            .id();

        world.run_system_once(think_system).unwrap();
        let action = world.get::<Action>(entity).unwrap();
        assert_eq!(*action, Action::Ascend);
    }
}
