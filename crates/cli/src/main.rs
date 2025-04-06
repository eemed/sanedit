mod logging;

use std::{fs, io, path::PathBuf};

use argh::FromArgs;
use sanedit_server::{Address, StartOptions};
use sanedit_terminal_client::{unix::UnixDomainSocketClient, SocketStartOptions};

/// command line options
#[derive(FromArgs)]
struct Cli {
    /// file to open
    #[argh(positional)]
    file: Option<PathBuf>,

    /// turn debugging information on
    #[argh(switch)]
    debug: bool,

    /// set configuration directory
    #[argh(option)]
    config_dir: Option<PathBuf>,

    /// set working directory
    #[argh(option)]
    working_dir: Option<PathBuf>,
}

fn main() {
    let cli: Cli = argh::from_env();

    logging::init_panic();
    logging::init_logger(cli.debug);

    // let open_files = cli.file.clone().map(|f| vec![f]).unwrap_or_default();
    let config_dir = cli.config_dir.clone();
    let working_dir = cli.working_dir.clone();
    let start_opts = StartOptions {
        config_dir,
        working_dir,
    };
    let socket_start_opts = SocketStartOptions { file: cli.file };

    let socket = PathBuf::from("/tmp/sanedit.sock");
    let exists = socket.try_exists().unwrap_or(false);
    if exists {
        log::info!("Connecting to existing socket..");
        // if socket already exists try to connect
        match UnixDomainSocketClient::connect(&socket) {
            Ok(socket) => {
                socket.run(socket_start_opts);
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
                socket.run(socket_start_opts);
            }
            Err(e) => {
                log::error!("{e}");
            }
        }
        join.join().unwrap()
    }

    let _ = fs::remove_file(socket);
}
