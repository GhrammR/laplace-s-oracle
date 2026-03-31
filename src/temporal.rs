//! Tau-Leaping temporal mechanics for the Laplace Oracle.
// Domain separation: no heap allocation may be introduced here.
#![allow(unknown_lints)]
#![deny(clippy::alloc_id)]
#![forbid(unsafe_code)]

use bevy_ecs::prelude::*;
use rand::Rng;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use crate::intelligence::{Action, TechnologyMask, NewlySpawned};

// ── Persistent RNG Resource ───────────────────────────────────────────────────

/// Deterministic ChaCha20 RNG persisted as a bevy_ecs Resource.
#[derive(Resource)]
pub struct RngResource {
    pub rng: ChaCha20Rng,
}

impl RngResource {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self { rng: ChaCha20Rng::from_seed(seed) }
    }

    /// 16-byte canonical serialisation of RNG word-position for world_hash.
    pub fn state_bytes(&self) -> [u8; 16] {
        self.rng.get_word_pos().to_le_bytes()
    }
}

// ── ECS Components ────────────────────────────────────────────────────────────

/// Biological/civilizational population count.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Population(pub u32);

/// Discrete civilizational index.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivIndex(pub u32);

/// 2D Coordinate in the world.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Position {
    pub x: u8,
    pub y: u8,
}

// ── GillespieTransition Trait ─────────────────────────────────────────────────

pub trait GillespieTransition {
    fn step(delta_years: u32, world: &mut World);
}

// ── TauLeap Engine ────────────────────────────────────────────────────────────

/// Zero-heap Tau-Leaping projection engine.
pub struct TauLeap;

impl GillespieTransition for TauLeap {
    fn step(delta_years: u32, world: &mut World) {
        let mut rng_res = world
            .remove_resource::<RngResource>()
            .expect("RngResource must be inserted before TauLeap::step");

        for _ in 0..delta_years {
            let env = *world.get_resource::<crate::physics::EnvironmentStack>()
                .expect("EnvironmentStack resource missing");

            let mut q =
                world.query::<(&mut Population, &mut TechnologyMask, &mut CivIndex, &Position, &Action)>();

            for (mut pop, tech, mut civ, pos, action) in q.iter_mut(world) {
                let p = pop.0 as f32;
                let t_bits = tech.bit_count() as f32;
                let t = (t_bits / 256.0).clamp(0.0_f32, 1.0_f32);

                // --- Malthusian Density Calculation ---
                let mut b_count = 0u32;
                let mut w_count = 0u32;
                
                let range = 1i16;
                for dy in -range..=range {
                    let ny = (pos.y as i16 + dy).rem_euclid(16) as usize;
                    let row_b = env.biomass[ny];
                    let row_w = env.water[ny];
                    for dx in -range..=range {
                        let nx = (pos.x as i16 + dx).rem_euclid(64) as usize;
                        if (row_b >> nx) & 1 == 1 { b_count += 1; }
                        if (row_w >> nx) & 1 == 1 { w_count += 1; }
                    }
                }

                let biomass_density = b_count as f32 / 9.0;
                let water_density = w_count as f32 / 9.0;
                let is_structured = (env.structure[pos.y as usize] >> pos.x) & 1 == 1;

                // Reaction rates
                let mut l_birth = (p * 0.03_f32 * (1.0_f32 + t)).min(50.0_f32);
                let mut l_death = (p * 0.02_f32).min(50.0_f32);
                let mut l_tech  = t * (1.0_f32 - t) * 0.5_f32;
                let l_civ       = (p * 0.001_f32 * (1.0_f32 + t)).min(20.0_f32);

                // Apply substrate coupling
                if water_density <= f32::EPSILON {
                    l_birth = 0.0;
                }

                if biomass_density > f32::EPSILON {
                    l_death *= 1.0 / biomass_density;
                } else {
                    l_death *= 10.0; // Exponential death cascade
                }

                if is_structured {
                    l_death *= 0.5; // Survival bonus
                }

                match action {
                    Action::Expand   => l_birth *= 1.5_f32,
                    Action::Research => l_tech  *= 2.0_f32,
                    Action::Defend   => l_death *= 0.5_f32,
                    Action::Flee     => l_death *= 1.2_f32, // Stress of fleeing
                    Action::Idle     => {},
                    _ => {},
                }

                let births = poisson_variate(&mut rng_res.rng, l_birth);
                let deaths = poisson_variate(&mut rng_res.rng, l_death);
                let _tech_d = poisson_variate(&mut rng_res.rng, l_tech);
                let civ_d  = poisson_variate(&mut rng_res.rng, l_civ);

                pop.0  = pop.0.saturating_add(births).saturating_sub(deaths);
                civ.0  = civ.0.saturating_add(civ_d);
            }
        }

        world.insert_resource(rng_res);
    }
}

// ── Knuth Poisson Sampler ─────────────────────────────────────────────────────

fn poisson_variate(rng: &mut ChaCha20Rng, lambda: f32) -> u32 {
    if lambda <= f32::EPSILON {
        return 0;
    }
    let l = (-lambda.min(50.0_f32)).exp();
    let mut k: u32 = 0;
    let mut p: f32 = 1.0_f32;
    loop {
        k = k.saturating_add(1);
        p *= rng.gen::<f32>();
        if p <= l || k >= 500 {
            break;
        }
    }
    k.saturating_sub(1)
}

use crate::intelligence::SimHashBrain;

// ── Natural Spawning System ───────────────────────────────────────────────────

pub fn natural_spawning_system(
    mut commands: Commands,
    mut rng: ResMut<RngResource>,
    pop_query: Query<&Population>,
    civ_query: Query<&CivIndex>,
) {
    let total_pop: u32 = pop_query.iter().map(|p| p.0).sum();

    if total_pop < 100 {
        let max_index = civ_query.iter().map(|c| c.0).max().unwrap_or(0);
        let next_id = max_index + 1;

        for _ in 0..100 {
            let x = rng.rng.gen_range(0..64);
            let y = rng.rng.gen_range(0..16);

            commands.spawn((
                Population(100),
                SimHashBrain(crate::ipc::MICROBE),
                Position { x, y },
                Action::Idle,
                CivIndex(next_id),
                TechnologyMask::default(),
                NewlySpawned,
            ));
        }
    }
}
