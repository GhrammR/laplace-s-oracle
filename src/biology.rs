//! Planetary Bitboard Substrate for the Laplace Oracle.
#![allow(unknown_lints)]
#![deny(clippy::alloc_id)]
#![forbid(unsafe_code)]

use crate::intelligence::EnvironmentData;
use crate::physics::{row_above, row_below, EnvironmentStack, WORLD_HEIGHT};
use bevy_ecs::prelude::{Component, ResMut};
use rayon::prelude::*;

// ── Components ─────────────────────────────────────────────────────────────────

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Taxonomy(pub u64);

// ── Constants ──────────────────────────────────────────────────────────────────

// Species:1, Family:Hominidae, Order:Primates
pub const HOMO_SAPIENS_TAXONOMY: u64 = (1 << 48) | (2 << 36) | (1 << 24);
// Species:2, Family:Canidae, Order:Carnivora
pub const CANIS_FAMILIARIS_TAXONOMY: u64 = (2 << 48) | (1 << 36);
// ── Taxonomic Decoder ──────────────────────────────────────────────────────────

/// Decodes a 64-bit SimHashBrain mask into a Linnaean taxonomic string.
///
/// Bit Slicing Contract:
/// - Bits 0-3 (Kingdom): 0 = Animalia, 1 = Plantae, 2 = Fungi, etc.
/// - Bits 4-11 (Phylum): 0 = Chordata, 1 = Arthropoda, etc.
/// - Bits 12-23 (Class): 0 = Mammalia, 1 = Reptilia, 2 = Dinosauria, 3 = Actinopterygii, etc.
/// - Bits 24-35 (Order): (Map a few basic ones like Carnivora, Primates, Salmoniformes, and default the rest to "Procedural Order X").
/// - Bits 36-47 (Family): (Map Canidae, Felidae, Hominidae, etc., default the rest).
/// - Bits 48-63 (Genus/Species): Hex representation (e.g., Species-A4F1).
pub fn decode_taxonomy(mask: u64) -> String {
    let kingdom = match mask & 0xF {
        0 => "Animalia",
        1 => "Plantae",
        2 => "Fungi",
        3 => "Protista",
        4 => "Archaea",
        5 => "Bacteria",
        _ => "Unknown Kingdom",
    };

    let phylum = match (mask >> 4) & 0xFF {
        0 => "Chordata",
        1 => "Arthropoda",
        2 => "Mollusca",
        3 => "Annelida",
        4 => "Cnidaria",
        _ => "Unknown Phylum",
    };

    let class = match (mask >> 12) & 0xFFF {
        0 => "Mammalia",
        1 => "Reptilia",
        2 => "Dinosauria",
        3 => "Actinopterygii",
        4 => "Aves",
        _ => "Unknown Class",
    };

    let order = match (mask >> 24) & 0xFFF {
        0 => "Carnivora",
        1 => "Primates",
        2 => "Salmoniformes",
        x => {
            return format!(
                "{} > {} > {} > Procedural Order {}",
                kingdom, phylum, class, x
            )
        }
    };

    let family = match (mask >> 36) & 0xFFF {
        0 => "Felidae",
        1 => "Canidae",
        2 => "Hominidae",
        x => {
            return format!(
                "{} > {} > {} > {} > Procedural Family {}",
                kingdom, phylum, class, order, x
            )
        }
    };

    let species = (mask >> 48) & 0xFFFF;

    format!(
        "{} > {} > {} > {} > {} > Species-{:X}",
        kingdom, phylum, class, order, family, species
    )
}

// ── LifeSystem ───────────────────────────────────────────────────────────────

/// Bevy system wrapper for life_step.
pub fn life_system(mut env_stack: ResMut<EnvironmentStack>, mut env_data: ResMut<EnvironmentData>) {
    life_step(&mut env_stack, &mut env_data);
}

/// Executes a bitwise cellular automata step across the Biomass layer of EnvironmentStack.
pub fn life_step(env_stack: &mut EnvironmentStack, env_data: &mut EnvironmentData) {
    let current = env_stack.biomass;
    let mut next = [0u64; WORLD_HEIGHT];

    // Parallelize processing using rayon for the row transitions.
    next.par_iter_mut().enumerate().for_each(|(i, target)| {
        // The biosphere lives on a 64x16 torus: horizontal neighbors rotate in-row and
        // vertical neighbors wrap across row 15 -> 0 and 0 -> 15.
        let prev = current[row_above(i)];
        let curr = current[i];
        let next_r = current[row_below(i)];

        // Horizontal shifts with wrap-around
        let l = |x: u64| x.rotate_left(1);
        let r = |x: u64| x.rotate_right(1);

        // ── Biomass Neighbors ──
        let n1 = l(prev);
        let n2 = prev;
        let n3 = r(prev);
        let n4 = l(curr);
        let n5 = r(curr);
        let n6 = l(next_r);
        let n7 = next_r;
        let n8 = r(next_r);

        // ── Water Proximity (Thirst Gate) ──
        let w_prev = env_stack.water[row_above(i)];
        let w_curr = env_stack.water[i];
        let w_next = env_stack.water[row_below(i)];

        let water_3x3 = l(w_prev)
            | w_prev
            | r(w_prev)
            | l(w_curr)
            | w_curr
            | r(w_curr)
            | l(w_next)
            | w_next
            | r(w_next);

        // Bitwise parallel sum (count 8 neighbors: b2 b1 b0)
        let (s0, c0) = half_adder(n1, n2);
        let (s1, c1) = full_adder(s0, n3, n4);
        let (s2, c2) = full_adder(s1, n5, n6);
        let (s3, c3) = full_adder(s2, n7, n8);

        let bit0 = s3;

        let (sa, ca) = half_adder(c0, c1);
        let (sb, cb) = full_adder(sa, c2, c3);

        let bit1 = sb;
        let bit2 = ca ^ cb;

        // B3 / S23 Rules + Thirst Gate + Photosynthesis Gate:
        let birth = !curr & !bit2 & bit1 & bit0 & water_3x3 & env_stack.light[i];
        let survival = curr & !bit2 & bit1 & water_3x3;

        *target = birth | survival;
    });

    env_stack.biomass = next;

    // Interface: Map Biomass population density to EnvironmentData (64-bit summary)
    let mut env_bits = 0u64;
    for (i, row) in next.iter().enumerate() {
        if row.count_ones() > 8 {
            env_bits |= 1 << i;
        }
        if (row & 0xFFFF_0000_0000_0000) > 0 {
            env_bits |= 1 << ((i + 16) % 64);
        }
    }
    env_data.0 = env_bits;
}

#[inline]
fn half_adder(a: u64, b: u64) -> (u64, u64) {
    (a ^ b, a & b)
}

#[inline]
fn full_adder(a: u64, b: u64, c: u64) -> (u64, u64) {
    let s = a ^ b ^ c;
    let carry = (a & b) | (c & (a ^ b));
    (s, carry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::EnvironmentData;
    use crate::physics::EnvironmentStack;

    #[test]
    fn test_drought_extinction() {
        let mut env_stack = EnvironmentStack::default();
        let mut env_data = EnvironmentData::default();

        env_stack.water = [0u64; 16];
        env_stack.biomass = [0u64; 16];
        env_stack.biomass[8] = 1u64 << 8;

        life_step(&mut env_stack, &mut env_data);

        assert_eq!(env_stack.biomass[8], 0u64);
    }

    #[test]
    fn test_photosynthesis_birth_requires_light() {
        let mut env_stack = EnvironmentStack::default();
        let mut env_data = EnvironmentData::default();

        env_stack.biomass = [0u64; WORLD_HEIGHT];
        env_stack.water = [0u64; WORLD_HEIGHT];
        env_stack.light = [0u64; WORLD_HEIGHT];

        env_stack.biomass[7] = (1u64 << 31) | (1u64 << 32);
        env_stack.biomass[8] = 1u64 << 31;

        env_stack.water[7] = (1u64 << 31) | (1u64 << 32) | (1u64 << 33);
        env_stack.water[8] = (1u64 << 31) | (1u64 << 32) | (1u64 << 33);
        env_stack.water[9] = (1u64 << 31) | (1u64 << 32) | (1u64 << 33);

        life_step(&mut env_stack, &mut env_data);
        assert_eq!(env_stack.biomass[8] & (1u64 << 32), 0);

        env_stack.biomass = [0u64; WORLD_HEIGHT];
        env_stack.biomass[7] = (1u64 << 31) | (1u64 << 32);
        env_stack.biomass[8] = 1u64 << 31;
        env_stack.light[8] = 1u64 << 32;

        life_step(&mut env_stack, &mut env_data);
        assert_eq!(env_stack.biomass[8] & (1u64 << 32), 1u64 << 32);
    }
}
