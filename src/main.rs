use base64::Engine;
use bevy_ecs::prelude::*;
use ed25519_dalek::SigningKey;
use laplace_oracle::{biology::Taxonomy, physics::*, intelligence::*, telemetry::*, temporal::*, ipc::*, StateVector};
use memmap2::MmapMut;
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{mem, time::SystemTime};

static RUNNING: AtomicBool = AtomicBool::new(true);

// ── System Phases ────────────────────────────────────────────────────────────

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum SimulationPhase {
    Think,
    Leap,
    Observation,
}

// ── Resources ────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct MmapResource(pub MmapMut);

unsafe impl Send for MmapResource {}
unsafe impl Sync for MmapResource {}

// ── Helper functions ─────────────────────────────────────────────────────────

fn open_universe_db(truncate: bool) -> MmapMut {
    const DB_SIZE: u64 = 1 << 30; // 1 GiB
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(truncate)
        .open("universe.db")
        .expect("open universe.db");
    if truncate {
        file.set_len(DB_SIZE).expect("set_len 1GiB");
    }
    unsafe { MmapMut::map_mut(&file).expect("mmap universe.db") }
}

pub fn init_miracles() -> MmapMut {
    const DB_SIZE: u64 = 4096;
    let file = open_miracle_file();
    file.set_len(DB_SIZE).expect("set_len 4096");
    unsafe { MmapMut::map_mut(&file).expect("mmap miracles.db") }
}

// ── Systems ──────────────────────────────────────────────────────────────────

fn hash_update_system(
    mmap: Res<MmapResource>,
    env: Res<EnvironmentStack>,
    rng: Res<RngResource>,
    tick: Res<Tick>,
    mut world_hash: ResMut<WorldHash>,
) {
    let mut hasher = Sha256::new();
    let slot_size = mem::size_of::<laplace_oracle::ArchivedStateVector>();
    
    // Persistence: Offset 0 = Tick, Offset 8 = StateVector
    let mmap_ref = &mmap.0;
    hasher.update(&tick.0.to_le_bytes());
    hasher.update(&mmap_ref[8..8+slot_size]);
    hasher.update(rng.state_bytes());
    
    // Hash EnvironmentStack (896 bytes)
    hasher.update(bytemuck::bytes_of(&env.biomass));
    hasher.update(bytemuck::bytes_of(&env.water));
    hasher.update(bytemuck::bytes_of(&env.temperature));
    hasher.update(bytemuck::bytes_of(&env.structure));
    hasher.update(bytemuck::bytes_of(&env.particle));
    hasher.update(bytemuck::bytes_of(&env.pressure));
    hasher.update(bytemuck::bytes_of(&env.microbiome));
    
    // Hash Memetics (8192 bytes)
    for word in &env.memetics {
        hasher.update(&word.to_le_bytes());
    }
    
    world_hash.0 = hasher.finalize().into();
}

fn tick_advance_system(mut tick: ResMut<Tick>) {
    tick.0 += 1;
}

fn miracle_grace_system(mut query: Query<(&mut Population, &mut MiracleGrace)>) {
    for (mut pop, mut grace) in query.iter_mut() {
        if pop.0 < 100 {
            pop.0 = 100;
        }
        grace.0 = grace.0.saturating_sub(1);
    }
}

fn miracle_grace_cleanup_system(mut commands: Commands, query: Query<(Entity, &MiracleGrace)>) {
    for (entity, grace) in query.iter() {
        if grace.0 == 0 {
            commands.entity(entity).remove::<MiracleGrace>();
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut interval = 1;
    let mut is_genesis = false;
    let mut is_archive = false;
    let mut seed_hash: Option<[u8; 32]> = None;
    let mut species = 0u64;
    let mut x = 0u8;
    let mut y = 0u8;

    for i in 1..args.len() {
        if args[i] == "--help" || args[i] == "-h" {
            println!("Laplace Oracle: Cosmological Simulation Engine");
            println!("");
            println!("Usage: laplace-oracle [OPTIONS]");
            println!("");
            println!("Options:");
            println!("  --interval <N>     Telemetry broadcast interval (default: 1)");
            println!("  --genesis          Initialize new universe with Genesis event");
            println!("  --archive          Create bit-perfect snapshot of current universe");
            println!("  --seed-hash <HEX>  Deterministic seed for simulation");
            println!("  --species <MASK>   Taxonomic mask for Genesis");
            println!("  --x <X>            X-coordinate for Genesis");
            println!("  --y <Y>            Y-coordinate for Genesis");
            println!("  --help, -h         Print this help message");
            std::process::exit(0);
        }
        if args[i] == "--interval" && i + 1 < args.len() {
            interval = args[i + 1].parse().unwrap_or(1);
        }
        if args[i] == "--genesis" {
            is_genesis = true;
        }
        if args[i] == "--archive" {
            is_archive = true;
        }
        if args[i] == "--seed-hash" && i + 1 < args.len() {
            let hex = &args[i + 1];
            if let Ok(bytes) = hex::decode(hex) {
                if bytes.len() == 32 {
                    let mut seed = [0u8; 32];
                    seed.copy_from_slice(&bytes);
                    seed_hash = Some(seed);
                }
            }
        }
        if args[i] == "--species" && i + 1 < args.len() {
            species = args[i + 1].parse().unwrap_or(0);
        }
        if args[i] == "--x" && i + 1 < args.len() {
            x = args[i + 1].parse().unwrap_or(0);
        }
        if args[i] == "--y" && i + 1 < args.len() {
            y = args[i + 1].parse().unwrap_or(0);
        }
    }

    if is_archive {
        let db = open_universe_db(false);
        let mut tick_bytes = [0u8; 8];
        tick_bytes.copy_from_slice(&db[0..8]);
        let current_tick = u64::from_le_bytes(tick_bytes);
        let dest_filename = format!("universe.db.tick_{}", current_tick);
        std::fs::copy("universe.db", &dest_filename).expect("archive copy");
        println!("ARCHIVED: State at tick {} saved to {}.", current_tick, dest_filename);
        std::process::exit(0);
    }

    // 1. Generate Key
    let initial_hash = seed_hash.unwrap_or([0u8; 32]);
    let signing_key = SigningKey::from_bytes(&initial_hash);
    let public_key = signing_key.verifying_key();
    let pk_b64 = base64::engine::general_purpose::STANDARD.encode(public_key.as_bytes());

    // 2. Persist Key for TUI Discovery
    std::fs::write("/tmp/oracle.pub", pk_b64.as_bytes()).expect("write /tmp/oracle.pub");

    // 3. Print Key to Stderr
    eprintln!("PUBLIC_KEY_B64: {}", pk_b64);

    // 4. Flush Stderr
    use std::io::Write;
    std::io::stderr().flush().unwrap();

    // 4. Pre-flight Sleep
    std::thread::sleep(std::time::Duration::from_millis(500));

    if is_genesis {
        let mut miracle_mmap = init_miracles();
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos() as u64;

        let cmd = MiracleCommand {
            nonce,
            miracle_type: MiracleType::Genesis as u8,
            target_x: x,
            target_y: y,
            radius: 1,
            payload: species,
        };

        miracle_mmap[0..20].copy_from_slice(&cmd.to_bytes());
        miracle_mmap.flush().unwrap();
        std::process::exit(0);
    }

    let stdout = std::io::stdout();

    let mut db = open_universe_db(is_genesis);
    let miracle_db = init_miracles();

    // 1. Initial State (Persistence: Tick at offset 0, StateVector at offset 8)
    {
        let tick = 0u64;
        db[0..8].copy_from_slice(&tick.to_le_bytes());
        
        let sv = StateVector { position: [0.0_f32; 3] };
        let bytes = rkyv::to_bytes::<_, 64>(&sv).expect("rkyv serialise");
        db[8..8 + bytes.len()].copy_from_slice(&bytes);
        db.flush_range(0, 8 + bytes.len()).expect("flush");
    }

    // 2. Init World
    let mut world = World::new();

    world.insert_resource(Tick(0));
    world.insert_resource(TelemetryInterval(interval));
    world.insert_resource(DroppedFrames(0));
    world.insert_resource(SigningKeyResource(signing_key.clone()));
    world.insert_resource(WorldHash::default());
    world.insert_resource(MmapResource(db));
    world.insert_resource(MiracleMmap(miracle_db));
    world.insert_resource(LastMiracleNonce::default());
    world.insert_resource(LastTick::default());
    world.insert_resource(RngResource::from_seed(initial_hash));
    world.insert_resource(EnvironmentData::default());
    world.insert_resource(EnvironmentStack::default());
    world.insert_resource(StdoutResource(stdout));
    world.insert_resource(TemporalState::default());

    world.spawn((
        WorldTag,
        Population(1000),
        Position { x: 0, y: 0 },
        TechnologyMask::default(),
        CivIndex(0),
        SimHashBrain(0x1234_5678_90AB_CDEF),
        Taxonomy(0), // Default taxonomy
        Action::Idle,
    ));

    let mut schedule = Schedule::default();
    schedule.configure_sets((
        SimulationPhase::Think,
        SimulationPhase::Leap,
        SimulationPhase::Observation,
    ).chain());

    schedule.add_systems((
        (genesis_listener_system, mutation_system, spatial_conflict_system, laplace_oracle::evolution::breeding_system, think_system, action_processing_system, memetics_system).chain().in_set(SimulationPhase::Think),

        (hazard_system, thermodynamics_system, microbiome_system, pressure_system, wind_system, vortex_system, volcanic_eruption_system, gravity_system, water_flow_system, hydrologic_cycle_system, laplace_oracle::events::world_event_system, laplace_oracle::biology::life_system, leap_system, natural_spawning_system, miracle_grace_system, miracle_grace_cleanup_system, hash_update_system, tick_advance_system).chain().in_set(SimulationPhase::Leap),
        observation_system.in_set(SimulationPhase::Observation),
    ));

    // 4. Infinite Simulation Loop
    ctrlc::set_handler(move || {
        RUNNING.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    while RUNNING.load(Ordering::SeqCst) {
        let tick_start = std::time::Instant::now();
        
        let (paused, speed_ms) = {
            let ts = world.resource::<TemporalState>();
            (ts.paused, ts.speed_ms)
        };

        if !paused {
            schedule.run(&mut world);
            
            // Persist current tick to disk
            let current_tick = world.resource::<Tick>().0;
            let mut db_res = world.resource_mut::<MmapResource>();
            db_res.0[0..8].copy_from_slice(&current_tick.to_le_bytes());
        } else {
            let mut sub_schedule = Schedule::default();
            sub_schedule.add_systems(genesis_listener_system);
            sub_schedule.run(&mut world);
        }
        
        let elapsed = tick_start.elapsed();
        let sleep_dur = std::time::Duration::from_millis(speed_ms);
        if elapsed < sleep_dur {
            std::thread::sleep(sleep_dur - elapsed);
        }
    }
}

fn leap_system(world: &mut World) {
    TauLeap::step(1, world);
}

#[derive(Component)]
struct WorldTag;
