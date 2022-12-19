pub mod addr;

pub mod types {
    include!(concat!(env!("OUT_DIR"), "/maestro.rs"));
}
