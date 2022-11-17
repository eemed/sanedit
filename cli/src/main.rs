use std::{path::PathBuf, thread};

use clap::{Parser, Subcommand};

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
    // Just run everything from here for now
    let cli = Cli::parse();

    let socket = PathBuf::from("/tmp/sanedit");
    let join = thread::spawn(|| {
        sanedit_editor::run_sync(vec![Address::UnixDomainSocket(socket)]);
    });

    // let ui =
    join.join().unwrap();
}
