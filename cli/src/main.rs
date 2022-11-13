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

fn spawn_server() -> thread::JoinHandle<()> {
    thread::spawn(|| {
    })
}

fn main() {
    let cli = Cli::parse();
    let (handle, join) = sanedit_editor::run(ListenAddr::UnixDomainSocket(PathBuf::from("/tmp/sanedit")));

    // let ui =
    // handle.join().unwrap();
}
