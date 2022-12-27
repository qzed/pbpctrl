use std::env;
use clap::CommandFactory;
use clap_complete::shells;

#[allow(dead_code)]
#[path = "src/cli.rs"]
mod cli;
use cli::*;


fn main() {
    let outdir = env::var_os("CARGO_TARGET_DIR")
        .or_else(|| env::var_os("OUT_DIR"))
        .unwrap();

    let mut cmd = Args::command();

    clap_complete::generate_to(shells::Bash, &mut cmd, "pbpctrl", &outdir).unwrap();
    clap_complete::generate_to(shells::Zsh, &mut cmd, "pbpctrl", &outdir).unwrap();
    clap_complete::generate_to(shells::Fish, &mut cmd, "pbpctrl", &outdir).unwrap();
}
