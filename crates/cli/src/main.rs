mod logging;

use core::time;
use std::{path::PathBuf, sync::mpsc, thread};

use clap::{Parser, Subcommand};
use sanedit_editor::Address;
use sanedit_terminal_client::unix::UnixDomainSocketClient;

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
    logging::setup();

    // Just run everything from here for now
    let cli = Cli::parse();

    let socket = PathBuf::from("/tmp/sanedit");
    let s = socket.clone();
    let join = sanedit_editor::run_sync(vec![Address::UnixDomainSocket(s)]);

    if let Some(join) = join {
        match UnixDomainSocketClient::connect(socket.clone()) {
            Ok(client) => client.run(),
            Err(e) => log::info!("Error connecting to socket: {}", e),
        }

        join.join().unwrap()
    }
}
