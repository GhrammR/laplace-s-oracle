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

use crate::telemetry::Tick;

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
    pub logic: [u64; 16],
    pub light: [u64; 16],
    pub elevation: [u8; 1024],
    pub memetics: [u64; 1024],
}

pub const WORLD_WIDTH: usize = 64;
pub const WORLD_HEIGHT: usize = 16;

#[inline]
pub const fn wrap_x(x: usize) -> usize {
    x % WORLD_WIDTH
}

#[inline]
pub const fn wrap_y(y: usize) -> usize {
    y % WORLD_HEIGHT
}

#[inline]
pub const fn row_above(row: usize) -> usize {
    (row + WORLD_HEIGHT - 1) % WORLD_HEIGHT
}

#[inline]
pub const fn row_below(row: usize) -> usize {
    (row + 1) % WORLD_HEIGHT
}

#[inline]
pub const fn cell_index(x: usize, y: usize) -> usize {
    wrap_y(y) * WORLD_WIDTH + wrap_x(x)
}

#[inline]
pub const fn bit_at(x: usize) -> u64 {
    1u64 << wrap_x(x)
}

#[inline]
pub fn elevation_at(env: &EnvironmentStack, x: usize, y: usize) -> u8 {
    env.elevation[cell_index(x, y)]
}

fn downhill_vertical_mask(env: &EnvironmentStack, from_row: usize, to_row: usize) -> u64 {
    let mut mask = 0u64;
    for x in 0..WORLD_WIDTH {
        if elevation_at(env, x, to_row) <= elevation_at(env, x, from_row) {
            mask |= bit_at(x);
        }
    }
    mask
}

fn lateral_flow_bits(env: &EnvironmentStack, row: usize, sources: u64, delta: isize) -> u64 {
    let mut out = 0u64;
    for x in 0..WORLD_WIDTH {
        let src_bit = bit_at(x);
        if sources & src_bit == 0 {
            continue;
        }
        let dest_x = ((x as isize + delta).rem_euclid(WORLD_WIDTH as isize)) as usize;
        if elevation_at(env, dest_x, row) <= elevation_at(env, x, row) {
            out |= bit_at(dest_x);
        }
    }
    out
}

fn lowland_bonus_mask(env: &EnvironmentStack, row: usize) -> u64 {
    let prev_row = row_above(row);
    let next_row = row_below(row);
    let mut mask = 0u64;
    for x in 0..WORLD_WIDTH {
        let center = elevation_at(env, x, row);
        let left_x = (x + WORLD_WIDTH - 1) % WORLD_WIDTH;
        let right_x = (x + 1) % WORLD_WIDTH;
        let neighbors = [
            elevation_at(env, left_x, row),
            elevation_at(env, right_x, row),
            elevation_at(env, x, prev_row),
            elevation_at(env, x, next_row),
        ];
        if neighbors.iter().all(|&n| center <= n) && neighbors.iter().any(|&n| center < n) {
            mask |= bit_at(x);
        }
    }
    mask
}

fn seasonal_light_width(orbital_row: usize) -> usize {
    match orbital_row / 4 {
        0 => 24,
        1 => 40,
        2 => 32,
        _ => 16,
    }
}

fn wrapped_band_mask(start: usize, width: usize) -> u64 {
    if width >= WORLD_WIDTH {
        return u64::MAX;
    }

    let mut mask = 0u64;
    for offset in 0..width {
        mask |= bit_at(start + offset);
    }
    mask
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
            logic: [0u64; 16],
            light: [0u64; 16],
            elevation: [0u8; 1024],
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
    let mut next_water = [0u64; WORLD_HEIGHT];

    for row in 0..WORLD_HEIGHT {
        let prev_row = row_above(row);
        let next_row = row_below(row);

        let w_curr = current.water[row];
        let w_above = current.water[prev_row];
        let s_curr = current.structure[row];
        let s_below = current.structure[next_row];
        let w_below = current.water[next_row];

        // Toroidal manifold: rows wrap vertically and bits wrap horizontally via rotate/mod arithmetic.
        let falling_from_above =
            w_above & !s_curr & downhill_vertical_mask(&current, prev_row, row);
        let blocked_at_current = w_curr & s_below;

        let supported = s_below | w_below;
        let can_spread = w_curr & supported;

        let spread_left = lateral_flow_bits(&current, row, can_spread, 1) & !s_curr & !w_curr;
        let spread_right = lateral_flow_bits(&current, row, can_spread, -1) & !s_curr & !w_curr;

        next_water[row] |= falling_from_above | blocked_at_current | spread_left | spread_right;
    }

    env.water = next_water;
}

pub fn stellar_system(tick: Res<Tick>, mut env: ResMut<EnvironmentStack>) {
    stellar_step(&mut env, tick.0);
}

pub fn stellar_step(env: &mut EnvironmentStack, tick: u64) {
    let orbital_row = ((tick / 1000) % WORLD_HEIGHT as u64) as usize;
    let hour = (tick % 1000) as usize;
    let width = seasonal_light_width(orbital_row);
    let drift_start = (hour * WORLD_WIDTH) / 1000;

    env.light = [0u64; WORLD_HEIGHT];
    env.light[orbital_row] = wrapped_band_mask(drift_start, width);
}

/// Hydrologic Cycle System: Evaporation-Precipitation cycle.
pub fn hydrologic_cycle_system(mut env: ResMut<EnvironmentStack>) {
    hydrologic_cycle_step(&mut env);
}

pub fn hydrologic_cycle_step(env: &mut EnvironmentStack) {
    let current = *env;
    let mut next_water = env.water;
    let mut next_pressure = env.pressure;
    let mut next_temperature = env.temperature;

    for i in 0..16 {
        let next_i = row_below(i);

        let w = current.water[i];
        let p = current.pressure[i];
        let heated = current.temperature[i] | (current.water[i] & current.light[i]);
        next_temperature[i] = heated;

        // Evaporation: Water + Heat -> Pressure
        let evaporated = w & heated;
        next_water[i] &= !evaporated;
        next_pressure[i] |= evaporated;

        // Precipitation: Pressure + !Heat -> Water (falls to row below)
        let condensed = p & !heated;
        next_pressure[i] &= !condensed;
        next_water[next_i] |= condensed;
    }

    env.water = next_water;
    env.pressure = next_pressure;
    env.temperature = next_temperature;
}

pub fn gravity_step(env: &mut EnvironmentStack) {
    let current = *env;
    let mut next_particle = [0u64; 16];

    for i in 0..16 {
        let prev_i = row_above(i);
        let next_i = row_below(i);

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
    let mut next_pressure = [0u64; WORLD_HEIGHT];

    for row in 0..WORLD_HEIGHT {
        let t = current.temperature[row];
        let s = current.structure[row];
        let gen = t & s;
        let neighbors =
            current.pressure[row].rotate_left(1) | current.pressure[row].rotate_right(1);
        let lowland_bias = lowland_bonus_mask(&current, row) & current.water[row];
        next_pressure[row] = current.pressure[row] | gen | neighbors | lowland_bias;
    }

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

pub fn computation_system(mut env: ResMut<EnvironmentStack>) {
    computation_step(&mut env);
}

pub fn computation_step(env: &mut EnvironmentStack) {
    let current = *env;
    let mut propagated = current.logic;
    let mut gate_outputs = [0u64; 16];

    for row in 0..16 {
        let prev_row = (row + 15) % 16;
        let next_row = (row + 1) % 16;
        let wire_mask =
            current.structure[prev_row] & current.structure[row] & current.structure[next_row];
        propagated[row] = wire_mask & (current.logic[row] | current.logic[prev_row]);
    }

    for row in 0..16 {
        let gate_bottom = (row + 1) % 16;
        let gate_output_row = (row + 2) % 16;
        let gate_anchor = current.structure[row]
            & current.structure[gate_bottom]
            & current.structure[row].rotate_right(1)
            & current.structure[gate_bottom].rotate_right(1);

        let input_left = current.logic[(row + 15) % 16] & gate_anchor;
        let input_right =
            (current.logic[(row + 15) % 16] & gate_anchor.rotate_left(1)).rotate_right(1);
        let active_inputs = input_left | input_right;
        let nand_output = active_inputs & !(input_left & input_right);

        let output_wire_mask = current.structure[(gate_output_row + 15) % 16]
            & current.structure[gate_output_row]
            & current.structure[(gate_output_row + 1) % 16];
        gate_outputs[gate_output_row] |= nand_output & output_wire_mask;
    }

    let mut next_logic = [0u64; 16];
    let mut next_biomass = current.biomass;
    for row in 0..16 {
        let desired = propagated[row] | gate_outputs[row];
        let toggles = (current.logic[row] ^ desired) & current.biomass[row];
        next_logic[row] = current.logic[row] ^ toggles;
        next_biomass[row] &= !toggles;
    }

    env.logic = next_logic;
    env.biomass = next_biomass;
}

/// Microbiome System: Conway's Game of Life (B3/S23) for microbial evolution.
/// Each tick, the microbiome substrate evolves bitwise.
pub fn microbiome_system(mut env: ResMut<EnvironmentStack>) {
    microbiome_step(&mut env);
}

pub fn microbiome_step(env: &mut EnvironmentStack) {
    let current = env.microbiome;
    let mut next = [0u64; WORLD_HEIGHT];

    for i in 0..WORLD_HEIGHT {
        let prev_i = row_above(i);
        let next_i = row_below(i);

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
    fn test_computation_step_nand() {
        let mut env = EnvironmentStack::default();
        env.biomass = [u64::MAX; 16];

        env.structure[0] = 0b11;
        env.structure[1] = 0b11;
        env.structure[2] = 0b11;
        env.structure[3] = 0b11;

        env.logic[15] = 0b01;

        computation_step(&mut env);

        assert_eq!(env.logic[2] & 0b01, 0b01);
        assert_eq!(env.biomass[2] & 0b01, 0);
    }

    #[test]
    fn test_microbiome_glider_wraps_horizontal() {
        let mut env = EnvironmentStack::default();
        env.microbiome = [0u64; WORLD_HEIGHT];
        env.microbiome[0] = bit_at(62);
        env.microbiome[1] = bit_at(63);
        env.microbiome[2] = bit_at(61) | bit_at(62) | bit_at(63);

        for _ in 0..4 {
            microbiome_step(&mut env);
        }

        assert_eq!(env.microbiome[1], bit_at(63));
        assert_eq!(env.microbiome[2], bit_at(0));
        assert_eq!(env.microbiome[3], bit_at(62) | bit_at(63) | bit_at(0));
    }

    #[test]
    fn test_microbiome_glider_wraps_vertical() {
        let mut env = EnvironmentStack::default();
        env.microbiome = [0u64; WORLD_HEIGHT];
        env.microbiome[14] = bit_at(2);
        env.microbiome[15] = bit_at(3);
        env.microbiome[0] = bit_at(1) | bit_at(2) | bit_at(3);

        for _ in 0..4 {
            microbiome_step(&mut env);
        }

        assert_eq!(env.microbiome[15], bit_at(3));
        assert_eq!(env.microbiome[0], bit_at(4));
        assert_eq!(env.microbiome[1], bit_at(2) | bit_at(3) | bit_at(4));
    }

    #[test]
    fn test_water_respects_elevation_gradient() {
        let mut env = EnvironmentStack::default();
        env.water = [0u64; WORLD_HEIGHT];
        env.structure = [0u64; WORLD_HEIGHT];
        env.biomass = [0u64; WORLD_HEIGHT];
        env.water[8] = bit_at(8);
        env.elevation[cell_index(8, 8)] = 10;
        env.elevation[cell_index(8, 9)] = 9;

        water_flow_step(&mut env);
        assert_eq!(env.water[9], bit_at(8));
    }

    #[test]
    fn test_pressure_prefers_lowlands() {
        let mut env = EnvironmentStack::default();
        env.pressure = [0u64; WORLD_HEIGHT];
        env.water[5] = bit_at(5);
        env.elevation[cell_index(4, 5)] = 4;
        env.elevation[cell_index(5, 5)] = 1;
        env.elevation[cell_index(6, 5)] = 4;
        env.elevation[cell_index(5, 4)] = 4;
        env.elevation[cell_index(5, 6)] = 4;

        pressure_step(&mut env);
        assert_eq!(env.pressure[5] & bit_at(5), bit_at(5));
    }

    #[test]
    fn test_stellar_band_tracks_orbit_and_drift() {
        let mut env = EnvironmentStack::default();
        stellar_step(&mut env, 4_500);

        assert!(env.light.iter().enumerate().all(|(row, bits)| if row == 4 {
            *bits != 0
        } else {
            *bits == 0
        }));
        assert_eq!(env.light[4].count_ones(), 40);
        assert_eq!(env.light[4] & bit_at(32), bit_at(32));
    }

    #[test]
    fn test_hydrologic_cycle_generates_heat_from_light() {
        let mut env = EnvironmentStack::default();
        env.water = [0u64; WORLD_HEIGHT];
        env.temperature = [0u64; WORLD_HEIGHT];
        env.pressure = [0u64; WORLD_HEIGHT];
        env.water[3] = bit_at(7);
        env.light[3] = bit_at(7);

        hydrologic_cycle_step(&mut env);

        assert_eq!(env.temperature[3] & bit_at(7), bit_at(7));
        assert_eq!(env.pressure[3] & bit_at(7), bit_at(7));
        assert_eq!(env.water[3] & bit_at(7), 0);
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
