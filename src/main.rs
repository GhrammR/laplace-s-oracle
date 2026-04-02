use base64::Engine;
use bevy_ecs::prelude::*;
use ed25519_dalek::SigningKey;
use laplace_oracle::{
    biology::Taxonomy, intelligence::*, ipc::*, physics::*, taxonomy_decoder::decode_taxonomy,
    telemetry::*, temporal::*, wormhole::*, StateVector,
};
use memmap2::MmapMut;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::os::unix::net::UnixDatagram;
use std::path::PathBuf;
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

fn init_ephemeral_mmap(size: usize) -> MmapMut {
    MmapMut::map_anon(size).expect("allocate anonymous mmap")
}

fn persist_tick(mmap: &mut MmapMut, tick: u64) {
    mmap[0..8].copy_from_slice(&tick.to_le_bytes());
}

fn scenario_summary(world: &mut World) -> serde_json::Value {
    let final_tick = world.resource::<Tick>().0;
    let world_hash = hex::encode(world.resource::<WorldHash>().0);

    let (global_biomass, global_temperature) = {
        let env = world.resource::<EnvironmentStack>();
        let biomass = env
            .biomass
            .iter()
            .map(|row| row.count_ones() as u64)
            .sum::<u64>();
        let temperature = env
            .temperature
            .iter()
            .map(|row| row.count_ones() as u64)
            .sum::<u64>();
        (biomass, temperature)
    };

    let mut apex_taxonomy = Taxonomy(0);
    let mut apex_technology_bits = 0u32;
    let mut apex_population = 0u32;

    let mut apex_query =
        world.query_filtered::<(&Population, &Taxonomy, &TechnologyMask), Without<WorldTag>>();
    for (population, taxonomy, technology) in apex_query.iter(world) {
        if population.0 > apex_population {
            apex_population = population.0;
            apex_taxonomy = *taxonomy;
            apex_technology_bits = technology.bit_count();
        }
    }

    if apex_population == 0 {
        let mut fallback_query = world.query::<(&Population, &Taxonomy, &TechnologyMask)>();
        for (population, taxonomy, technology) in fallback_query.iter(world) {
            if population.0 > apex_population {
                apex_population = population.0;
                apex_taxonomy = *taxonomy;
                apex_technology_bits = technology.bit_count();
            }
        }
    }

    json!({
        "final_tick": final_tick,
        "world_hash": world_hash,
        "apex_species": decode_taxonomy(apex_taxonomy.0),
        "global_biomass": global_biomass,
        "global_temperature": global_temperature,
        "technology_bits": apex_technology_bits,
    })
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
    hasher.update(tick.0.to_le_bytes());
    hasher.update(&mmap_ref[8..8 + slot_size]);
    hasher.update(rng.state_bytes());

    // Hash EnvironmentStack layers
    hasher.update(bytemuck::bytes_of(&env.biomass));
    hasher.update(bytemuck::bytes_of(&env.water));
    hasher.update(bytemuck::bytes_of(&env.temperature));
    hasher.update(bytemuck::bytes_of(&env.structure));
    hasher.update(bytemuck::bytes_of(&env.particle));
    hasher.update(bytemuck::bytes_of(&env.pressure));
    hasher.update(bytemuck::bytes_of(&env.microbiome));
    hasher.update(bytemuck::bytes_of(&env.logic));
    hasher.update(bytemuck::bytes_of(&env.light));
    hasher.update(bytemuck::bytes_of(&env.elevation));
    hasher.update(bytemuck::bytes_of(&env.geology));

    // Hash Memetics (8192 bytes)
    for word in &env.memetics {
        hasher.update(word.to_le_bytes());
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
    let mut wormhole_rx: Option<PathBuf> = None;
    let mut wormhole_tx: Option<PathBuf> = None;
    let mut scenario_output: Option<PathBuf> = None;
    let mut scenario_duration: Option<u64> = None;
    let api_socket_path = PathBuf::from("/tmp/oracle_api.sock");

    for i in 1..args.len() {
        if args[i] == "--help" || args[i] == "-h" {
            println!("Laplace Oracle: Cosmological Simulation Engine");
            println!();
            println!("Usage: laplace-oracle [OPTIONS]");
            println!();
            println!("Options:");
            println!("  --interval <N>     Telemetry broadcast interval (default: 1)");
            println!("  --genesis          Initialize new universe with Genesis event");
            println!("  --archive          Create bit-perfect snapshot of current universe");
            println!("  --seed-hash <HEX>  Deterministic seed for simulation");
            println!("  --species <MASK>   Taxonomic mask for Genesis");
            println!("  --x <X>            X-coordinate for Genesis");
            println!("  --y <Y>            Y-coordinate for Genesis");
            println!("  --wormhole-rx <SOCKET_PATH>  Bind a non-blocking incoming wormhole socket");
            println!("  --wormhole-tx <SOCKET_PATH>  Send outgoing ascension payloads to a wormhole socket");
            println!("  --scenario <OUTPUT_FILE.json>  Run headless and export a JSON summary");
            println!("  --duration <TICKS>  Required with --scenario; number of ticks to execute");
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
        if args[i] == "--wormhole-rx" && i + 1 < args.len() {
            wormhole_rx = Some(PathBuf::from(&args[i + 1]));
        }
        if args[i] == "--wormhole-tx" && i + 1 < args.len() {
            wormhole_tx = Some(PathBuf::from(&args[i + 1]));
        }
        if args[i] == "--scenario" && i + 1 < args.len() {
            scenario_output = Some(PathBuf::from(&args[i + 1]));
        }
        if args[i] == "--duration" && i + 1 < args.len() {
            scenario_duration = args[i + 1].parse().ok();
        }
    }

    let scenario_mode = scenario_output.is_some();
    if scenario_mode && scenario_duration.is_none() {
        eprintln!("ERROR: --scenario requires --duration <TICKS>");
        std::process::exit(1);
    }

    if is_archive {
        let db = open_universe_db(false);
        let mut tick_bytes = [0u8; 8];
        tick_bytes.copy_from_slice(&db[0..8]);
        let current_tick = u64::from_le_bytes(tick_bytes);
        let dest_filename = format!("universe.db.tick_{}", current_tick);
        std::fs::copy("universe.db", &dest_filename).expect("archive copy");
        println!(
            "ARCHIVED: State at tick {} saved to {}.",
            current_tick, dest_filename
        );
        std::process::exit(0);
    }

    // 1. Generate Key
    let initial_hash = seed_hash.unwrap_or([0u8; 32]);
    let signing_key = SigningKey::from_bytes(&initial_hash);
    let public_key = signing_key.verifying_key();
    let pk_b64 = base64::engine::general_purpose::STANDARD.encode(public_key.as_bytes());

    if !scenario_mode {
        // 2. Persist Key for TUI Discovery
        std::fs::write("/tmp/oracle.pub", pk_b64.as_bytes()).expect("write /tmp/oracle.pub");

        // 3. Print Key to Stderr
        eprintln!("PUBLIC_KEY_B64: {}", pk_b64);

        // 4. Flush Stderr
        use std::io::Write;
        std::io::stderr().flush().unwrap();

        // 5. Pre-flight Sleep
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

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

    let stdout = if scenario_mode {
        None
    } else {
        Some(std::io::stdout())
    };

    let api_listener = if scenario_mode {
        None
    } else {
        Some(bind_api_listener(&api_socket_path).expect("bind oracle api listener"))
    };

    let rx_socket = if scenario_mode {
        None
    } else {
        wormhole_rx.as_ref().map(|path| {
            let _ = std::fs::remove_file(path);
            let socket = UnixDatagram::bind(path).expect("bind wormhole-rx");
            socket
                .set_nonblocking(true)
                .expect("set_nonblocking wormhole-rx");
            socket
        })
    };
    let tx_socket = if scenario_mode {
        None
    } else {
        wormhole_tx.as_ref().map(|_| {
            let socket = UnixDatagram::unbound().expect("bind wormhole-tx");
            socket
                .set_nonblocking(true)
                .expect("set_nonblocking wormhole-tx");
            socket
        })
    };

    let mut db = if scenario_mode {
        init_ephemeral_mmap(4096)
    } else {
        open_universe_db(is_genesis)
    };
    let miracle_db = if scenario_mode {
        init_ephemeral_mmap(4096)
    } else {
        init_miracles()
    };

    // 1. Initial State (Persistence: Tick at offset 0, StateVector at offset 8)
    {
        let tick = 0u64;
        db[0..8].copy_from_slice(&tick.to_le_bytes());

        let sv = StateVector {
            position: [0.0_f32; 3],
        };
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
    if let Some(api_listener) = api_listener {
        world.insert_resource(ApiListenerRuntime {
            listener: api_listener,
            path: api_socket_path.clone(),
        });
    }
    world.insert_resource(LastMiracleNonce::default());
    world.insert_resource(LastTick::default());
    world.insert_resource(RngResource::from_seed(initial_hash));
    world.insert_resource(EnvironmentData::default());
    world.insert_resource(EnvironmentStack::default());
    world.insert_resource(CelestialSeed::default());
    if let Some(stdout) = stdout {
        world.insert_resource(StdoutResource(stdout));
    }
    world.insert_resource(TemporalState::default());
    world.insert_resource(WormholeActivity::default());
    world.insert_resource(WormholeRuntime {
        rx: rx_socket,
        tx: tx_socket,
        tx_path: wormhole_tx,
        rx_path: wormhole_rx.clone(),
    });

    world.spawn((
        WorldTag,
        Population(1000),
        Position { x: 0, y: 0 },
        TechnologyMask::default(),
        CivIndex(0),
        SimHashBrain(0x1234_5678_90AB_CDEF),
        Taxonomy(0), // Default taxonomy
        linguistic_sequence_from_taxonomy(Taxonomy(0)),
        Action::Idle,
    ));

    let mut init_schedule = Schedule::default();
    init_schedule.add_systems(hash_update_system);
    init_schedule.run(&mut world);
    let tick_zero_hash = world.resource::<WorldHash>().0;
    world.insert_resource(celestial_seed_from_hash(tick_zero_hash));

    let mut schedule = Schedule::default();
    schedule.configure_sets(
        (
            SimulationPhase::Think,
            SimulationPhase::Leap,
            SimulationPhase::Observation,
        )
            .chain(),
    );

    schedule.add_systems((
        (
            api_listener_system,
            genesis_listener_system,
            arrival_system,
            mutation_system,
            linguistic_trade_system,
            spatial_conflict_system,
            laplace_oracle::evolution::breeding_system,
            think_system,
            action_processing_system,
            memetics_system,
        )
            .chain()
            .in_set(SimulationPhase::Think),
        (
            (
                hazard_system,
                thermodynamics_system,
                homeostasis_system,
                tectonic_system,
                geology_system,
                microbiome_system,
                pressure_system,
                computation_system,
                orbital_system,
                wind_system,
                vortex_system,
                volcanic_eruption_system,
                gravity_system,
                water_flow_system,
            ),
            (
                hydrologic_cycle_system,
                laplace_oracle::events::world_event_system,
                laplace_oracle::biology::life_system,
                leap_system,
                natural_spawning_system,
                consolidation_system,
                ascension_system,
                miracle_grace_system,
                miracle_grace_cleanup_system,
                hash_update_system,
                tick_advance_system,
            ),
        )
            .chain()
            .in_set(SimulationPhase::Leap),
    ));

    if !scenario_mode {
        schedule.add_systems(observation_system.in_set(SimulationPhase::Observation));
    }

    if scenario_mode {
        let duration = scenario_duration.expect("scenario duration");
        for _ in 0..duration {
            schedule.run(&mut world);
            let current_tick = world.resource::<Tick>().0;
            let mut db_res = world.resource_mut::<MmapResource>();
            persist_tick(&mut db_res.0, current_tick);
        }

        let report = scenario_summary(&mut world);
        let output_path = scenario_output.expect("scenario output path");
        let json_output = serde_json::to_string_pretty(&report).expect("serialize scenario report");
        std::fs::write(&output_path, json_output).expect("write scenario report");
        return;
    }

    // 4. Infinite Simulation Loop
    let wormhole_rx_shutdown = wormhole_rx.clone();
    let api_socket_shutdown = api_socket_path.clone();
    ctrlc::set_handler(move || {
        if let Some(path) = wormhole_rx_shutdown.as_ref() {
            let _ = std::fs::remove_file(path);
        }
        let _ = std::fs::remove_file(&api_socket_shutdown);
        RUNNING.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

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
            persist_tick(&mut db_res.0, current_tick);
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

    let _ = std::fs::remove_file(&api_socket_path);
}

fn leap_system(world: &mut World) {
    TauLeap::step(1, world);
}

#[derive(Component)]
struct WorldTag;
