use std::{path::PathBuf, thread};

use clap::{Parser, Subcommand};
use sanedit_editor::ListenAddr;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// File to open
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long)]
    debug: bool,
}

fn main() {
    let socket = PathBuf::from("/tmp/sanedit");
    let cli = Cli::parse();

    let s = socket.clone();
    let join = thread::spawn(|| {
        sanedit_editor::run_sync(ListenAddr::UnixDomainSocket(s));
    });

    // let ui =
    join.join().unwrap();
}
