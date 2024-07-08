mod logging;

use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use sanedit_editor::{Address, StartOptions};
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

    /// Set configuration directory
    #[arg(short, long, value_name = "DIRECTORY")]
    config_dir: Option<PathBuf>,

    /// Set working directory
    #[arg(short, long, value_name = "DIRECTORY")]
    working_dir: Option<PathBuf>,
}

fn main() {
    logging::setup();

    let cli = Cli::parse();
    let open_files = cli.file.clone().map(|f| vec![f]).unwrap_or(vec![]);
    let config_dir = cli.config_dir.clone();
    let working_dir = cli.working_dir.clone();
    let start_opts = StartOptions {
        open_files,
        config_dir,
        working_dir,
    };

    let socket = PathBuf::from("/tmp/sanedit.sock");
    let exists = socket.try_exists().unwrap_or(false);
    if exists {
        // if socket already exists try to connect
        connect(&socket);
    } else {
        // If no socket startup server
        let s = socket.clone();
        let addrs = vec![Address::UnixDomainSocket(s)];
        let join = sanedit_editor::run_sync(addrs, start_opts);
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
