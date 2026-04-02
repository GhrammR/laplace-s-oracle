pub mod biology;
pub mod events;
pub mod evolution;
pub mod intelligence;
pub mod ipc;
pub mod physics;
pub mod taxonomy_decoder;
pub mod telemetry;
pub mod temporal;

use rkyv::{Archive, Serialize};

#[derive(Archive, Serialize)]
#[archive(check_bytes)]
#[repr(C)]
pub struct StateVector {
    pub position: [f32; 3],
}
