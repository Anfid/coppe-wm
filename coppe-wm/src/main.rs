use std::sync::Arc;
pub use x11rb::rust_connection::RustConnection as X11Conn;

mod bindings;
mod client;
mod events;
mod layout;
mod runner;
mod wm;

use crate::runner::Runner;
use crate::wm::WindowManager;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let (command_tx, command_rx) = std::sync::mpsc::sync_channel(50);
    let mut runner = Runner::new(event_rx, command_tx);

    let (conn, screen_num) = X11Conn::connect(None).unwrap();
    let conn = Arc::new(conn);

    std::thread::spawn(move || runner.run());

    let mut wm = WindowManager::init(conn.clone(), screen_num, event_tx, command_rx).unwrap();

    wm.run(&*conn).unwrap();
}
