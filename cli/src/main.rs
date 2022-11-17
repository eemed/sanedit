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

#[tokio::main]
fn main() {
    // Just run everything from here for now
    let cli = Cli::parse();

    let socket = PathBuf::from("/tmp/sanedit");
    let editor_join = tokio::spawn(sanedit_editor::run(vec![Address::UnixDomainSocket(socket)]));

    // TODO
    // Client. tokio or just threads?
    // Need channel between read write task and logic task
    // Write can be used anywhere when connected and got the sender
    //
    // Read task
    // Read bytes -> messages
    //
    // Input task
    // Another write messages -> bytes
    //
    // Logic task
    // Get messages -> draw / other logic

    editor_join.join().unwrap();
}
