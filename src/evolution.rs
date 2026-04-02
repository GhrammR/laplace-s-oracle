//! Evolution Engine for the Laplace Oracle.
//!
//! Implements Darwinian crossover and environmentally-catalyzed mutation.
//! Adheres to the 100MB RAM footprint and memory asceticism mandates.

use crate::biology::Taxonomy;
use crate::intelligence::{
    linguistic_sequence_from_taxonomy, Action, LinguisticSequence, SimHashBrain, TechnologyMask,
};
use crate::physics::EnvironmentStack;
use crate::temporal::{Population, Position, RngResource};
use bevy_ecs::prelude::*;
use rand::Rng;

// ── Crossover Logic ───────────────────────────────────────────────────────────

/// Half-mask crossover: child = (p1_upper | p2_lower)
#[inline]
pub fn half_mask_crossover(p1: u64, p2: u64) -> u64 {
    (p1 & 0xFFFFFFFF00000000) | (p2 & 0x00000000FFFFFFFF)
}

// ── Mutation Logic ────────────────────────────────────────────────────────────

/// Mutation: Bit-flip probability proportional to environment temperature.
/// A temperature bit of '1' guarantees at least one bit-flip.
#[inline]
pub fn mutate(brain: u64, temperature_at_pos: bool, rng: &mut impl Rng) -> u64 {
    let mut next_brain = brain;

    if temperature_at_pos {
        // Guarantee at least one flip
        let bit = rng.gen_range(0..64);
        next_brain ^= 1 << bit;

        // Additional flips proportional to "heat" (simplified as additional random flips)
        // In a bitboard world, 'temperature_at_pos' being true means high entropy.
        for _ in 0..rng.gen_range(0..3) {
            let b = rng.gen_range(0..64);
            next_brain ^= 1 << b;
        }
    } else {
        // Low probability background mutation
        if rng.gen_bool(0.0001) {
            let bit = rng.gen_range(0..64);
            next_brain ^= 1 << bit;
        }
    }

    next_brain
}

// ── Breeding System ───────────────────────────────────────────────────────────

/// Breeding System: Entities at the same location with same taxonomic Class breed.
/// Runs in SimulationPhase::Think.
pub fn breeding_system(
    mut commands: Commands,
    mut rng_res: ResMut<RngResource>,
    env: Res<EnvironmentStack>,
    query: Query<(Entity, &Position, &Taxonomy, &SimHashBrain)>,
) {
    // 8GB LAW: Avoid unbounded heap.
    // We'll use a fixed-size buffer or a very careful transient collection.
    // Given Bevy's current patterns in this codebase, we'll collect and sort.
    let mut entities: Vec<_> = query.iter().collect();
    if entities.is_empty() {
        return;
    }

    // Sort by position to find adjacencies (here: same coordinate for performance)
    entities.sort_by_key(|&(_, pos, _, _)| (pos.x, pos.y));

    let mut i = 0;
    while i < entities.len() {
        let (e1, pos1, tax1, brain1) = entities[i];

        // Find others at the same position
        let mut j = i + 1;
        while j < entities.len() {
            let (e2, pos2, tax2, brain2) = entities[j];
            if pos1.x != pos2.x || pos1.y != pos2.y {
                break;
            }

            // Check eligibility:
            // 1. biomass > 0 at coordinates
            let biomass_present = (env.biomass[pos1.y as usize] >> pos1.x) & 1 == 1;

            // 2. same taxonomic Class (bits 12-23)
            let class_mask = 0x0000_0000_00FF_F000;
            let same_class = (tax1.0 & class_mask) == (tax2.0 & class_mask);

            if biomass_present && same_class && e1 != e2 {
                // Breeding triggered
                let mut child_tax = half_mask_crossover(tax1.0, tax2.0);
                let raw_child_brain = half_mask_crossover(brain1.0, brain2.0);

                // ── THE MICROBIAL BUTTERFLY EFFECT ──
                // Read the microbiome bit at the spawn coordinate.
                let micro_bit = (env.microbiome[pos1.y as usize] >> pos1.x) & 1 == 1;
                if micro_bit {
                    // Apply deterministic XOR mask: 0x0000_0000_0000_1000 (flip 12th bit)
                    child_tax ^= 0x0000_0000_0000_1000;
                }

                let temp_present = (env.temperature[pos1.y as usize] >> pos1.x) & 1 == 1;
                let child_brain = mutate(raw_child_brain, temp_present, &mut rng_res.rng);

                // Spawn Child
                let child_taxonomy = Taxonomy(child_tax);
                let child_language = linguistic_sequence_from_taxonomy(child_taxonomy);
                commands.spawn((
                    Population(1),
                    Action::Idle,
                    Position {
                        x: pos1.x,
                        y: pos1.y,
                    },
                    child_taxonomy,
                    LinguisticSequence(child_language.0),
                    SimHashBrain(child_brain),
                    TechnologyMask::default(),
                ));

                // Only one child per pair per tick to avoid population explosion
                // Skip to next potential parent block
                break;
            }
            j += 1;
        }
        i = j;
    }
}
