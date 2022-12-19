use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["proto/pw.rpc.packet.proto"], &["proto/"])?;
    Ok(())
}
