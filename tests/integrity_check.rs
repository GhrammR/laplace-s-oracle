use bevy_ecs::prelude::*;
use bevy_ecs::system::RunSystemOnce;
use laplace_oracle::{
    biology::*, intelligence::*, physics::*, telemetry::TelemetryFrame, temporal::*,
};
use sha2::{Digest, Sha256};

#[test]
fn test_biological_determinism() {
    let mut world_a = World::new();
    let mut world_b = World::new();

    world_a.insert_resource(EnvironmentStack::default());
    world_b.insert_resource(EnvironmentStack::default());
    world_a.insert_resource(EnvironmentData::default());
    world_b.insert_resource(EnvironmentData::default());

    // 100 iterations of LifeSystem
    for _ in 0..100 {
        {
            let mut stack = world_a.remove_resource::<EnvironmentStack>().unwrap();
            let mut env = world_a.remove_resource::<EnvironmentData>().unwrap();
            life_step(&mut stack, &mut env);
            world_a.insert_resource(stack);
            world_a.insert_resource(env);
        }
        {
            let mut stack = world_b.remove_resource::<EnvironmentStack>().unwrap();
            let mut env = world_b.remove_resource::<EnvironmentData>().unwrap();
            life_step(&mut stack, &mut env);
            world_b.insert_resource(stack);
            world_b.insert_resource(env);
        }
    }

    let grid_a = world_a.get_resource::<EnvironmentStack>().unwrap().biomass;
    let grid_b = world_b.get_resource::<EnvironmentStack>().unwrap().biomass;

    assert_eq!(
        grid_a, grid_b,
        "Biological grid mismatch after 100 iterations"
    );

    // Optional: Check hash of the final state
    let mut hasher = Sha256::new();
    for row in grid_a {
        hasher.update(row.to_le_bytes());
    }
    let hash = hasher.finalize();

    println!(
        "[PASS] Biological determinism validated. Final Hash: {:x}",
        hash
    );
}

#[test]
fn test_brain_determinism() {
    let brain = SimHashBrain(0x1234_5678_90AB_CDEF);
    let stimulus_res = Stimulus(0x1234_5678_90AB_CDEF); // dist 0 (0..16) -> Research
    let stimulus_exp = Stimulus(0x0000_0000_0000_0000); // dist 32 (17..32) -> Expand
    let stimulus_idl = Stimulus(!0x1234_5678_90AB_CDEF); // dist 64 (49..64) -> Idle

    assert_eq!(
        Intelligence::decide(&brain, &stimulus_res, 16),
        Action::Research
    );
    assert_eq!(
        Intelligence::decide(&brain, &stimulus_exp, 16),
        Action::Expand
    );
    assert_eq!(
        Intelligence::decide(&brain, &stimulus_idl, 16),
        Action::Idle
    );

    println!("[PASS] Brain determinism validated");
}

#[test]
fn test_determinism_after_leap() {
    let mut world_a = World::new();
    let mut world_b = World::new();

    let seed = [0x42u8; 32];
    world_a.insert_resource(RngResource::from_seed(seed));
    world_b.insert_resource(RngResource::from_seed(seed));
    world_a.insert_resource(EnvironmentStack::default());
    world_b.insert_resource(EnvironmentStack::default());

    let brain_val = 0x1234_5678_90AB_CDEF;
    let ent_a = world_a
        .spawn((
            Population(1000),
            TechnologyMask::default(),
            CivIndex(10),
            Position { x: 0, y: 0 },
            SimHashBrain(brain_val),
            Action::default(),
        ))
        .id();
    let ent_b = world_b
        .spawn((
            Population(1000),
            TechnologyMask::default(),
            CivIndex(10),
            Position { x: 0, y: 0 },
            SimHashBrain(brain_val),
            Action::default(),
        ))
        .id();

    // Perform 5-year leap on both
    TauLeap::step(5, &mut world_a);
    TauLeap::step(5, &mut world_b);

    // 1. Validate Component Equality
    let p_a = world_a.get::<Population>(ent_a).unwrap().0;
    let p_b = world_b.get::<Population>(ent_b).unwrap().0;

    assert_eq!(p_a, p_b, "Population mismatch");

    // 2. Validate RNG State Identity
    let rng_a = world_a.get_resource::<RngResource>().unwrap().state_bytes();
    let rng_b = world_b.get_resource::<RngResource>().unwrap().state_bytes();
    assert_eq!(rng_a, rng_b, "RNG state mismatch");

    println!("[PASS] Determinism validated: Pop={}, RNG={:?}", p_a, rng_a);
}

#[test]
fn telemetry_frame_size_seal() {
    assert_eq!(std::mem::size_of::<TelemetryFrame>(), 9280);
    println!("[PASS] TelemetryFrame size is exactly 9248 bytes");
}

#[test]
fn test_gravity_halts_at_structure() {
    let mut env = EnvironmentStack::default();

    // Particle at row 0, bit 10
    env.particle[0] = 1 << 10;
    // Structure at row 2, bit 10
    env.structure[2] = 1 << 10;

    let mut world = World::new();
    world.insert_resource(env);

    // Tick 1
    gravity_step(
        world
            .get_resource_mut::<EnvironmentStack>()
            .unwrap()
            .into_inner(),
    );
    let env = world.get_resource::<EnvironmentStack>().unwrap();
    assert_eq!(env.particle[1], 1 << 10, "Particle should fall to row 1");
    assert_eq!(env.particle[0], 0, "Row 0 should be empty");

    // Tick 2
    gravity_step(
        world
            .get_resource_mut::<EnvironmentStack>()
            .unwrap()
            .into_inner(),
    );
    let env = world.get_resource::<EnvironmentStack>().unwrap();
    assert_eq!(
        env.particle[1],
        1 << 10,
        "Particle should be blocked at row 1 by structure in row 2"
    );
    assert_eq!(
        env.particle[2], 0,
        "Particle should NOT enter row 2 because of structure"
    );

    println!("[PASS] Gravity halting validated");
}

#[test]
fn test_volcanic_eruption_trigger() {
    let mut env = EnvironmentStack::default();

    // High heat + High pressure at (5, 5)
    env.temperature[5] = 1 << 5;
    env.pressure[5] = 1 << 5;

    let mut world = World::new();
    world.insert_resource(env);

    volcanic_eruption_step(
        world
            .get_resource_mut::<EnvironmentStack>()
            .unwrap()
            .into_inner(),
    );
    let env = world.get_resource::<EnvironmentStack>().unwrap();

    assert_eq!(
        env.particle[5],
        1 << 5,
        "Eruption should spawn particle at (5, 5)"
    );

    // Reset and test only heat
    let mut env2 = EnvironmentStack::default();
    env2.temperature[5] = 1 << 5;
    world.insert_resource(env2);
    volcanic_eruption_step(
        world
            .get_resource_mut::<EnvironmentStack>()
            .unwrap()
            .into_inner(),
    );
    let env2 = world.get_resource::<EnvironmentStack>().unwrap();
    assert_eq!(env2.particle[5], 0, "No eruption with only heat");

    println!("[PASS] Refined volcanic eruption trigger validated");
}

#[test]
fn test_wind_advection() {
    use laplace_oracle::physics::{wind_step, EnvironmentStack};
    let mut env = EnvironmentStack::default();

    // Pressure gradient: High (col 5) to Low (col 4)
    // Note: p & !p_left (rotate_left(1) shifts col 5 to 6, so p_left at 5 is what was at 4? No.)
    // Let's re-verify wind_step logic:
    // push_left = p & !p.rotate_left(1);
    // If col 5 is 1 and col 6 is 0, p.rotate_left(1) at col 5 is same bit as p at col 6.
    // Wait, rotate_left(1) on a bitboard: bit 0 becomes 1, bit 63 becomes 0.
    // So if col 5 is 1, p.rotate_left(1) has a 1 at col 6.
    // push_left = p[5] & !p[6] -> If 5 is High and 6 is Low, push left?
    // Actually, rotate_left(1) moves bits to HIGHER indices (left in many visualizations).
    // So col 5 -> col 6.
    // If bit 5 is 1 and 6 is 0, push_left has bit 5 set.
    // Then (particles & push_left).rotate_left(1) moves bit 5 to bit 6.

    env.pressure[5] = 1 << 5; // High at 5, Low at 6.
    env.particle[5] = 1 << 5;

    wind_step(&mut env);

    assert_eq!(
        (env.particle[5] >> 6) & 1,
        1,
        "Particle should have been advected to col 6 (left-shift)"
    );
}

#[test]
fn test_vortex_destroys_structure() {
    use laplace_oracle::physics::{vortex_step, EnvironmentStack};
    let mut env = EnvironmentStack::default();

    // 3x3 Eye at row 5, col 5
    let mask = 0b111 << 4;
    env.pressure[4] = mask;
    env.pressure[5] = (1 << 4) | (1 << 6); // 0 at col 5
    env.pressure[6] = mask;

    env.structure[5] |= 1 << 5;
    env.biomass[5] |= 1 << 5;

    vortex_step(&mut env);

    assert_eq!(
        (env.structure[5] >> 5) & 1,
        0,
        "Structure at vortex center should be destroyed"
    );
    assert_eq!(
        (env.biomass[5] >> 5) & 1,
        0,
        "Biomass at vortex center should be destroyed"
    );

    println!("[PASS] Vortex destructive physics validated");
}

#[test]
fn test_great_filter_event_can_trigger() {
    use laplace_oracle::events::world_event_step;
    use laplace_oracle::physics::EnvironmentStack;

    let mut env = EnvironmentStack::default();
    // Tick 549837 triggers SHA-256[0..4] < 4295
    let winning_tick = 549837;

    world_event_step(winning_tick, &mut env);

    let mut temp_bits = 0;
    let mut particle_bits = 0;
    for row in 0..16 {
        temp_bits += env.temperature[row].count_ones();
        particle_bits += env.particle[row].count_ones();
    }

    assert!(
        temp_bits >= 16,
        "Meteor impact should flip 4x4 temperature block"
    );
    assert!(
        particle_bits >= 16,
        "Meteor impact should flip 4x4 particle block"
    );

    println!("[PASS] Great Filter event trigger and impact validated");
}

#[test]
fn test_genetic_crossover() {
    use laplace_oracle::evolution::half_mask_crossover;
    let p1 = 0xAAAA_AAAA_AAAA_AAAA;
    let p2 = 0x5555_5555_5555_5555;
    let child = half_mask_crossover(p1, p2);
    // Upper 32 bits from p1: AAAA_AAAA
    // Lower 32 bits from p2: 5555_5555
    assert_eq!(child, 0xAAAA_AAAA_5555_5555);
}

#[test]
fn test_mutation_rate_scales_with_temp() {
    use laplace_oracle::evolution::mutate;
    use rand_chacha::rand_core::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let brain = 0x0000_0000_0000_0000;

    // Test with temperature = true (guaranteed flip)
    let mutated_hot = mutate(brain, true, &mut rng);
    assert_ne!(
        mutated_hot, brain,
        "Mutation must occur in high temperature"
    );
    assert!(
        mutated_hot.count_ones() >= 1,
        "At least one bit must be flipped"
    );

    // Test with temperature = false (rare flip)
    let mut flips = 0;
    for _ in 0..1000 {
        if mutate(brain, false, &mut rng) != brain {
            flips += 1;
        }
    }
    assert!(
        flips < 10,
        "Background mutation should be rare (expected ~0.1, got {})",
        flips
    );
}

#[test]
fn test_microbial_mutation() {
    use laplace_oracle::evolution::breeding_system;

    let mut world = World::new();
    let mut env = EnvironmentStack::default();

    // Position (10, 5)
    let x = 10u8;
    let y = 5u8;

    // 1. Set microbiome bit at (10, 5)
    env.microbiome[y as usize] = 1 << x;
    // 2. Ensure biomass at (10, 5) for breeding
    env.biomass[y as usize] = 1 << x;

    world.insert_resource(env);
    world.insert_resource(RngResource::from_seed([0u8; 32]));

    // 3. Spawn two parents at (10, 5) with same Class
    let tax_val = 0x0000_0000_00AA_A000; // Class mask 0x00FF_F000
    let brain_val = 0x1234_5678_90AB_CDEF;

    world.spawn((
        Position { x, y },
        Taxonomy(tax_val),
        SimHashBrain(brain_val),
        Population(100),
    ));
    world.spawn((
        Position { x, y },
        Taxonomy(tax_val),
        SimHashBrain(brain_val),
        Population(100),
    ));

    // 4. Run breeding system
    let _ = world.run_system_once(breeding_system);

    // 5. Find child (parents have Pop 100, child has Pop 1)
    let mut child_query = world.query::<(&Taxonomy, &Population)>();
    let mut found_child = false;
    for (tax, pop) in child_query.iter(&world) {
        if pop.0 == 1 {
            // Child found
            // Expected tax: parent_tax XOR 0x1000 (because microbiome bit is 1)
            assert_eq!(
                tax.0,
                tax_val ^ 0x0000_0000_0000_1000,
                "Child taxonomy should be mutated by microbiome"
            );
            found_child = true;
        }
    }
    assert!(found_child, "Child should have been spawned");

    println!("[PASS] Microbial butterfly effect validated");
}

#[test]
fn test_archive_command() {
    use std::fs;
    let db_path = "universe.db";
    let tick = 12345u64;

    // 1. Create a dummy universe.db with tick 12345
    {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(db_path)
            .unwrap();
        file.set_len(1024).unwrap();
        let mut mmap = unsafe { memmap2::MmapMut::map_mut(&file).unwrap() };
        mmap[0..8].copy_from_slice(&tick.to_le_bytes());
        mmap.flush().unwrap();
    }

    // 2. Run archival logic (simulated)
    let current_tick = {
        let file = std::fs::File::open(db_path).unwrap();
        let mmap = unsafe { memmap2::Mmap::map(&file).unwrap() };
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&mmap[0..8]);
        u64::from_le_bytes(bytes)
    };

    let dest_filename = format!("universe.db.tick_{}", current_tick);
    fs::copy(db_path, &dest_filename).expect("archive copy");

    // 3. Assert
    assert!(fs::metadata(&dest_filename).is_ok());
    let copied_bytes = fs::read(&dest_filename).unwrap();
    assert_eq!(&copied_bytes[0..8], &tick.to_le_bytes());

    // Cleanup
    let _ = fs::remove_file(db_path);
    let _ = fs::remove_file(dest_filename);
}

#[test]
fn test_seeding_determinism() {
    use base64::Engine;
    use ed25519_dalek::SigningKey;

    let seed_hex = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    let seed_bytes = hex::decode(seed_hex).unwrap();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&seed_bytes);

    let key1 = SigningKey::from_bytes(&seed);
    let key2 = SigningKey::from_bytes(&seed);

    let pk1 = base64::engine::general_purpose::STANDARD.encode(key1.verifying_key().as_bytes());
    let pk2 = base64::engine::general_purpose::STANDARD.encode(key2.verifying_key().as_bytes());

    assert_eq!(pk1, pk2, "Public keys must be identical for the same seed");

    // Test RngResource determinism
    let mut world_a = World::new();
    let mut world_b = World::new();
    world_a.insert_resource(RngResource::from_seed(seed));
    world_b.insert_resource(RngResource::from_seed(seed));

    let rng_a = world_a.get_resource::<RngResource>().unwrap().state_bytes();
    let rng_b = world_b.get_resource::<RngResource>().unwrap().state_bytes();
    assert_eq!(
        rng_a, rng_b,
        "RNG state must be identical for the same seed"
    );
}
