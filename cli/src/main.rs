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

fn spawn_server() -> thread::JoinHandle<()> {
    thread::spawn(|| {
        let server = sanedit_editor::server::run();
    })
}

fn main() {
    let cli = Cli::parse();

    let handle = spawn_server();
    let ui = 
}
