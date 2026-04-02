//! Thermodynamics Substrate for the Laplace Oracle.
//!
//! Enforces:
//! 1. Bitwise Cellular Automata for physical interactions.
//! 2. Zero-copy state transitions.
//! 3. Rayon-parallelized environment updates.

#![deny(clippy::all)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::identity_op)]
#![allow(clippy::field_reassign_with_default)]

use bevy_ecs::prelude::*;
use rayon::prelude::*;
use sha2::{Digest, Sha256};

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct EnvironmentStack {
    pub biomass: [u64; 16],
    pub water: [u64; 16],
    pub temperature: [u64; 16],
    pub structure: [u64; 16],
    pub particle: [u64; 16],
    pub pressure: [u64; 16],
    pub microbiome: [u64; 16],
    pub memetics: [u64; 1024],
}

impl Default for EnvironmentStack {
    fn default() -> Self {
        let mut biomass = [0u64; 16];
        // Seed some biomass
        biomass[7] = 0x0000_0000_FFFF_FFFF;
        biomass[8] = 0x0000_0000_FFFF_FFFF;

        let mut microbiome = [0u64; 16];
        // Seed some initial microbial life (Acorn pattern-ish)
        microbiome[8] = 0x0000_0000_0001_0000;
        microbiome[9] = 0x0000_0000_0000_0100;
        microbiome[10] = 0x0000_0000_0011_1011;

        Self {
            biomass,
            water: [0u64; 16],
            temperature: [0u64; 16],
            structure: [0u64; 16],
            particle: [0u64; 16],
            pressure: [0u64; 16],
            microbiome,
            memetics: [0u64; 1024],
        }
    }
}

/// Memetics System: Spread of cultural information.
/// Runs in SimulationPhase::Think.
pub fn memetics_system(
    mut env: ResMut<EnvironmentStack>,
    query: Query<&crate::temporal::Position>,
) {
    // Collect all entity positions
    let mut positions = Vec::new();
    for pos in query.iter() {
        positions.push((pos.x as usize, pos.y as usize));
    }

    let mut next_memetics = env.memetics;

    for &(x, y) in &positions {
        let idx = y * 64 + x;
        let meme = env.memetics[idx];
        if meme != 0 {
            // Check neighbor at (x+1, y)
            let nx = (x + 1) % 64;
            let n_idx = y * 64 + nx;

            // Propagation condition: source has meme, destination doesn't
            if env.memetics[n_idx] == 0 {
                // Determine if an entity is at the destination to receive it
                let receiver_present = positions.iter().any(|&(px, py)| px == nx && py == y);
                if receiver_present {
                    next_memetics[n_idx] = meme;
                }
            }
        }
    }

    env.memetics = next_memetics;
}

/// Thermodynamics System: Fire Spread and Consumption.
/// Runs in SimulationPhase::Leap.
pub fn thermodynamics_system(mut env: ResMut<EnvironmentStack>) {
    let current = *env;
    let mut next_temp = [0u64; 16];
    let mut next_biomass = [0u64; 16];

    // 1. Parallel Temperature Spread (Fire Spread)
    // Temperature spreads to adjacent bits IF the adjacent bit has Biomass.
    next_temp
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, target_temp)| {
            let prev_idx = (i + 15) % 16;
            let next_idx = (i + 1) % 16;

            let t_prev = current.temperature[prev_idx];
            let t_curr = current.temperature[i];
            let t_next = current.temperature[next_idx];

            let l = |x: u64| x.rotate_left(1);
            let r = |x: u64| x.rotate_right(1);

            // Combined neighborhood of temperature
            let neighbors = l(t_prev)
                | t_prev
                | r(t_prev)
                | l(t_curr)
                | r(t_curr)
                | l(t_next)
                | t_next
                | r(t_next);

            // Temperature spreads only where biomass is present
            // Also keep existing temperature (it might cool down later, but for now it's static/additive)
            *target_temp = (t_curr | neighbors) & current.biomass[i];
        });

    // 2. Consumption: Biomass bits are flipped to 0 if corresponding Temperature bit is 1.
    for i in 0..16 {
        next_biomass[i] = current.biomass[i] & !next_temp[i];
    }

    env.temperature = next_temp;
    env.biomass = next_biomass;
}

/// Gravity System: Newtonian advection for particles.
pub fn gravity_system(mut env: ResMut<EnvironmentStack>) {
    gravity_step(&mut env);
}

/// Water Flow System: Bitwise fluid dynamics (Gravity + Lateral).
pub fn water_flow_system(mut env: ResMut<EnvironmentStack>) {
    water_flow_step(&mut env);
}

pub fn water_flow_step(env: &mut EnvironmentStack) {
    let current = *env;
    let mut next_water = [0u64; 16];

    for i in 0..16 {
        let prev_i = (i + 15) % 16;
        let next_i = (i + 1) % 16;

        let w_curr = current.water[i];
        let w_above = current.water[prev_i];
        let s_curr = current.structure[i];
        let s_below = current.structure[next_i];
        let w_below = current.water[next_i];

        // 1. Gravity: Water bits move down if no structure below
        let falling_from_above = w_above & !s_curr;
        let blocked_at_current = w_curr & s_below;

        // 2. Lateral Flow: If supported (structure or water below), spread left/right
        let supported = s_below | w_below;
        let can_spread = w_curr & supported;

        let l = |x: u64| x.rotate_left(1);
        let r = |x: u64| x.rotate_right(1);

        let spread_left = l(can_spread) & !s_curr & !w_curr;
        let spread_right = r(can_spread) & !s_curr & !w_curr;

        // Water stays if it's blocked below OR if it didn't spread (simplified for bitwise)
        // Actually, bits move, so we must be careful not to create water.
        // For lateral flow in a bitboard, we "leak" into neighbors.

        next_water[i] |= falling_from_above | blocked_at_current | spread_left | spread_right;
    }

    env.water = next_water;
}

/// Hydrologic Cycle System: Evaporation-Precipitation cycle.
pub fn hydrologic_cycle_system(mut env: ResMut<EnvironmentStack>) {
    hydrologic_cycle_step(&mut env);
}

pub fn hydrologic_cycle_step(env: &mut EnvironmentStack) {
    let current = *env;
    let mut next_water = env.water;
    let mut next_pressure = env.pressure;

    for i in 0..16 {
        let next_i = (i + 1) % 16;

        let w = current.water[i];
        let t = current.temperature[i];
        let p = current.pressure[i];

        // Evaporation: Water + Heat -> Pressure
        let evaporated = w & t;
        next_water[i] &= !evaporated;
        next_pressure[i] |= evaporated;

        // Precipitation: Pressure + !Heat -> Water (falls to row below)
        let condensed = p & !t;
        next_pressure[i] &= !condensed;
        next_water[next_i] |= condensed;
    }

    env.water = next_water;
    env.pressure = next_pressure;
}

pub fn gravity_step(env: &mut EnvironmentStack) {
    let current = *env;
    let mut next_particle = [0u64; 16];

    for i in 0..16 {
        let prev_i = (i + 15) % 16;
        let next_i = (i + 1) % 16;

        let blocked = current.particle[i] & current.structure[next_i];
        let falling_from_above = current.particle[prev_i] & !current.structure[i];
        next_particle[i] = blocked | falling_from_above;
    }

    env.particle = next_particle;
}

/// Pressure System: Simple CA for atmospheric pressure.
pub fn pressure_system(mut env: ResMut<EnvironmentStack>) {
    pressure_step(&mut env);
}

pub fn pressure_step(env: &mut EnvironmentStack) {
    let current = *env;
    let mut next_pressure = [0u64; 16];

    next_pressure
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, target)| {
            let t = current.temperature[i];
            let s = current.structure[i];
            let gen = t & s;
            let l = |x: u64| x.rotate_left(1);
            let r = |x: u64| x.rotate_right(1);
            let neighbors = l(current.pressure[i]) | r(current.pressure[i]);
            *target = current.pressure[i] | gen | neighbors;
        });

    env.pressure = next_pressure;
}

/// Volcanic Eruption System: Particle seeding from Heat + Pressure.
pub fn volcanic_eruption_system(mut env: ResMut<EnvironmentStack>) {
    volcanic_eruption_step(&mut env);
}

pub fn volcanic_eruption_step(env: &mut EnvironmentStack) {
    let current = *env;
    let mut next_particle = env.particle;

    for i in 0..16 {
        let t = current.temperature[i];
        let p = current.pressure[i];
        let eruption_points = t & p;
        next_particle[i] |= eruption_points;
    }

    env.particle = next_particle;
}

/// Thermodynamic Lethality: Despawn entities on fire.
pub fn hazard_system(
    mut commands: Commands,
    env: Res<EnvironmentStack>,
    query: Query<(Entity, &crate::temporal::Position)>,
) {
    for (entity, pos) in query.iter() {
        let x = pos.x as usize;
        let y = pos.y as usize;
        if y < 16 && x < 64 {
            let fire = (env.temperature[y] >> x) & 1 == 1;
            if fire {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Wind System: Bitwise advection for particles and temperature based on pressure gradients.
pub fn wind_system(mut env: ResMut<EnvironmentStack>) {
    wind_step(&mut env);
}

pub fn wind_step(env: &mut EnvironmentStack) {
    let current = *env;
    for i in 0..16 {
        let p = current.pressure[i];
        let p_left = p.rotate_left(1);
        let p_right = p.rotate_right(1);

        // Gradient: High to Low
        let push_left = p & !p_left;
        let push_right = p & !p_right;

        // Advect Particles
        let particles = current.particle[i];
        let p_moved_left = (particles & push_left).rotate_left(1);
        let p_moved_right = (particles & push_right).rotate_right(1);
        let p_stayed = particles & !push_left & !push_right;
        env.particle[i] = p_stayed | p_moved_left | p_moved_right;

        // Advect Heat (Temperature)
        let temp = current.temperature[i];
        let t_moved_left = (temp & push_left).rotate_left(1);
        let t_moved_right = (temp & push_right).rotate_right(1);
        let t_stayed = temp & !push_left & !push_right;
        env.temperature[i] = t_stayed | t_moved_left | t_moved_right;
    }
}

/// Vortex System: 3x3 destructive rotational shift for tornadoes.
pub fn vortex_system(mut env: ResMut<EnvironmentStack>) {
    vortex_step(&mut env);
}

pub fn vortex_step(env: &mut EnvironmentStack) {
    let current = *env;
    let mut centers = [0u64; 16];

    for i in 0..16 {
        let p_prev = current.pressure[(i + 15) % 16];
        let p_curr = current.pressure[i];
        let p_next = current.pressure[(i + 1) % 16];

        // Pattern: 0 surrounded by 1s (Eye of the storm)
        let mask = (p_prev & p_prev.rotate_left(1) & p_prev.rotate_right(1))
            & (p_curr.rotate_left(1) & p_curr.rotate_right(1) & !p_curr)
            & (p_next & p_next.rotate_left(1) & p_next.rotate_right(1));
        centers[i] = mask;
    }

    // Lethal Consequences: Clear 3x3 around every vortex center
    for i in 0..16 {
        let c_prev = centers[(i + 15) % 16];
        let c_curr = centers[i];
        let c_next = centers[(i + 1) % 16];

        let destroy_mask = (c_prev | c_prev.rotate_left(1) | c_prev.rotate_right(1))
            | (c_curr | c_curr.rotate_left(1) | c_curr.rotate_right(1))
            | (c_next | c_next.rotate_left(1) | c_next.rotate_right(1));

        env.biomass[i] &= !destroy_mask;
        env.structure[i] &= !destroy_mask;
    }
}

/// Microbiome System: Conway's Game of Life (B3/S23) for microbial evolution.
/// Each tick, the microbiome substrate evolves bitwise.
pub fn microbiome_system(mut env: ResMut<EnvironmentStack>) {
    let current = env.microbiome;
    let mut next = [0u64; 16];

    for i in 0..16 {
        let prev_i = (i + 15) % 16;
        let next_i = (i + 1) % 16;

        let m_prev = current[prev_i];
        let m_curr = current[i];
        let m_next = current[next_i];

        let l = |x: u64| x.rotate_left(1);
        let r = |x: u64| x.rotate_right(1);

        // Neighbors
        let x1 = l(m_prev);
        let x2 = m_prev;
        let x3 = r(m_prev);
        let x4 = l(m_curr);
        let x5 = r(m_curr);
        let x6 = l(m_next);
        let x7 = m_next;
        let x8 = r(m_next);

        // Bitwise 8-bit sum (S2 S1 S0) using cascades of full/half-adders
        let fa = |a: u64, b: u64, c: u64| {
            let s = a ^ b ^ c;
            let cy = (a & b) | (b & c) | (c & a);
            (s, cy)
        };
        let ha = |a: u64, b: u64| {
            let s = a ^ b;
            let cy = a & b;
            (s, cy)
        };

        let (s_a, c_a) = fa(x1, x2, x3);
        let (s_b, c_b) = fa(x4, x5, x6);
        let (s_c, c_c) = ha(x7, x8);

        let (s0, c_d) = fa(s_a, s_b, s_c); // Plane 0 sum
        let (s_e, c_e) = fa(c_a, c_b, c_c);
        let (s1, c_f) = ha(s_e, c_d); // Plane 1 sum
        let s2 = c_e ^ c_f; // Plane 2 sum (ignore carry, max sum is 8)

        // Conway's Rules:
        // Birth (B3): dead cell with 3 neighbors.
        // Survival (S23): live cell with 2 or 3 neighbors.
        // Bitwise: (!s2 & s1) & (s0 | m_curr)
        next[i] = (s1 & !s2) & (s0 | m_curr);
    }

    env.microbiome = next;
}

pub fn local_memetics_signature(
    env: &EnvironmentStack,
    pos: &crate::temporal::Position,
) -> [u64; 4] {
    let idx = pos.y as usize * 64 + pos.x as usize;
    let local = env.memetics[idx];
    if local == 0 {
        return [0u64; 4];
    }

    let mut hasher = Sha256::new();
    hasher.update(local.to_le_bytes());
    hasher.update([pos.x, pos.y]);
    let digest = hasher.finalize();
    let mut words = [0u64; 4];
    for (i, chunk) in digest.chunks_exact(8).enumerate() {
        words[i] = u64::from_le_bytes(chunk.try_into().unwrap());
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::Position;

    #[test]
    fn test_memetic_infection() {
        let mut world = World::new();
        world.insert_resource(EnvironmentStack::default());

        // Spawn entity A (source) at (10, 10)
        world.spawn(Position { x: 10, y: 10 });
        // Spawn entity B (receiver) at (11, 10)
        world.spawn(Position { x: 11, y: 10 });

        // Set a meme hash at (10, 10)
        {
            let mut env = world.resource_mut::<EnvironmentStack>();
            env.memetics[10 * 64 + 10] = 0xDEADBEEF;
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(memetics_system);
        schedule.run(&mut world);

        let env = world.resource::<EnvironmentStack>();
        assert_eq!(env.memetics[10 * 64 + 11], 0xDEADBEEF);
    }

    #[test]
    fn test_water_gravity() {
        let mut env = EnvironmentStack::default();
        // Clear all initial water/biomass
        env.water = [0u64; 16];
        env.biomass = [0u64; 16];
        env.structure = [0u64; 16];

        // Place water at (8,8)
        env.water[8] = 1 << 8;

        // Step
        water_flow_step(&mut env);

        // Assert water moves to (8,9)
        assert_eq!(env.water[9], 1 << 8);
        assert_eq!(env.water[8], 0);
    }
}
