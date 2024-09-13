mod logging;

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use clap::Parser;
use sanedit_server::{Address, StartOptions};
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
        log::info!("Connecting to existing socket..");
        // if socket already exists try to connect
        match UnixDomainSocketClient::connect(&socket) {
            Ok(socket) => {
                socket.run();
                return;
            }
            Err(e) => match e.kind() {
                io::ErrorKind::ConnectionRefused => {}
                _ => {
                    log::error!("{e}");
                    return;
                }
            },
        }
    }

    log::info!("Creating a new socket..");
    // If no socket startup server
    let s = socket.clone();
    let addrs = vec![Address::UnixDomainSocket(s)];
    let join = sanedit_editor::run_sync(addrs, start_opts);
    if let Some(join) = join {
        match UnixDomainSocketClient::connect(&socket) {
            Ok(socket) => {
                socket.run();
            }
            Err(e) => {
                log::error!("{e}");
            }
        }
        join.join().unwrap()
    }

    let _ = fs::remove_file(socket);
}
