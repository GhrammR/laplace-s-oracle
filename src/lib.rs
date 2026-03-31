pub mod temporal;
pub mod intelligence;
pub mod biology;
pub mod physics;
pub mod telemetry;
pub mod ipc;
pub mod events;
pub mod evolution;
pub mod taxonomy_decoder;


use rkyv::{Archive, Serialize};

#[derive(Archive, Serialize)]
#[archive(check_bytes)]
#[repr(C)]
pub struct StateVector {
    pub position: [f32; 3],
}
