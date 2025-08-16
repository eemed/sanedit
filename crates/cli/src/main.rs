mod logging;

use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    io,
    path::PathBuf,
    time::SystemTime,
};

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

    /// connect to an existing instance
    #[argh(option)]
    connect: Option<PathBuf>,

    /// set log file location
    #[argh(option)]
    log_file: Option<PathBuf>,
}

fn main() {
    let cli: Cli = argh::from_env();

    // let open_files = cli.file.clone().map(|f| vec![f]).unwrap_or_default();
    let config_dir = cli.config_dir.clone();
    let working_dir = cli.working_dir.clone();
    let socket_hash = {
        let cwd = std::env::current_dir();
        let dir = working_dir
            .as_ref()
            .or(cwd.as_ref().ok())
            .expect("No working directory found");
        let mut hasher = DefaultHasher::new();
        dir.as_os_str().hash(&mut hasher);
        if cli.debug {
            "-debug".hash(&mut hasher)
        }
        if let Ok(n) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            n.hash(&mut hasher);
        }
        format!("{:x}", hasher.finish())
    };
    let tmp = sanedit_core::tmp_dir();
    if tmp.is_none() {
        eprintln!("TMP directory not accessible");
        return;
    }
    let tmp = tmp.unwrap();

    let log_file = cli
        .log_file
        .unwrap_or_else(|| tmp.join(format!("sanedit-{socket_hash}.log")));
    logging::init_panic();
    logging::init_logger(&log_file, cli.debug);

    let connect = cli.connect.is_some();
    let socket = cli
        .connect
        .unwrap_or_else(|| PathBuf::from(format!("/tmp/{socket_hash}-sanedit.sock")));
    let socket_start_opts = SocketStartOptions { file: cli.file };
    let try_connect = connect || socket.try_exists().unwrap_or(false);
    if try_connect {
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

    let start_opts = StartOptions {
        config_dir,
        working_dir,
        debug: cli.debug,
        addr: Address::UnixDomainSocket(socket.clone()),
    };
    log::info!("Creating a new socket..");
    // If no socket startup server
    let join = sanedit_editor::run_sync(start_opts);
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
    let _ = fs::remove_file(&log_file);
}
