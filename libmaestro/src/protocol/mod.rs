pub mod addr;
pub mod codec;

pub mod types {
    include!(concat!(env!("OUT_DIR"), "/maestro_pw.rs"));
}
