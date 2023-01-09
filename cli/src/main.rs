mod logging;

use core::time;
use std::{path::PathBuf, thread};

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
    let editor_join =
        thread::spawn(|| sanedit_editor::run_sync(vec![Address::UnixDomainSocket(s)]));

    thread::sleep(time::Duration::from_millis(100));

    match UnixDomainSocketClient::connect(socket) {
        Ok(client) => {
            client.run();
        }
        Err(e) => {
            log::info!("Error connecting to socket: {}", e);
        }
    }

    editor_join.join().unwrap();
}
