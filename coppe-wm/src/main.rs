use clap::{App, AppSettings, ArgMatches, SubCommand};
use std::{
    sync::{mpsc, Arc},
    thread,
};

pub(crate) use x11rb::rust_connection::RustConnection as X11Conn;

mod bindings;
mod client;
mod events;
mod layout;
mod runner;
mod state;
mod wm;

use crate::runner::Runner;
use crate::state::State;
use crate::wm::WindowManager;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let matches = App::new("Coppe WM")
        .version("0.0.1")
        .about("Window manager fully configurable with WASM plugins")
        .global_setting(AppSettings::ColoredHelp)
        .subcommand(
            SubCommand::with_name("plugin")
                .about("Manage plugins")
                .setting(AppSettings::SubcommandRequired)
                .subcommand(
                    SubCommand::with_name("init")
                        .about("Initialize default plugins in $HOME directory"),
                ),
        )
        .subcommand(SubCommand::with_name("test").about("Run WM tests"))
        .get_matches();

    match matches.subcommand() {
        ("plugin", Some(matches)) => manage_plugins(matches),
        ("test", Some(_)) => todo!(),
        _ => {
            run_wm();
        }
    }
}

fn manage_plugins(matches: &ArgMatches) {
    match matches.subcommand() {
        ("init", Some(_matches)) => todo!(),
        _ => {}
    }
}

fn run_wm() {
    let state = State::default();
    let (event_tx, event_rx) = mpsc::channel();
    let (command_tx, command_rx) = mpsc::sync_channel(50);
    let mut runner = Runner::new(state.clone(), event_rx, command_tx);

    let (conn, screen_num) = X11Conn::connect(None).unwrap();
    let conn = Arc::new(conn);

    thread::spawn(move || runner.run());

    let mut wm =
        WindowManager::init(conn.clone(), screen_num, state, event_tx, command_rx).unwrap();

    wm.run(&*conn).unwrap();
}
