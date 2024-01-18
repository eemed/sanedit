mod logging;

use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
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
    let _cli = Cli::parse();

    let socket = PathBuf::from("/tmp/sanedit.sock");
    let exists = socket.try_exists().unwrap_or(false);
    if exists {
        // if socket already exists try to connect
        connect(&socket);
    } else {
        // If no socket startup server
        let s = socket.clone();
        let join = sanedit_editor::run_sync(vec![Address::UnixDomainSocket(s)]);
        if let Some(join) = join {
            connect(&socket);
            join.join().unwrap()
        }

        let _ = fs::remove_file(socket);
    }
}

fn connect(socket: &Path) {
    match UnixDomainSocketClient::connect(socket) {
        Ok(client) => client.run(),
        Err(e) => log::info!("Error connecting to socket: {}", e),
    }
}
