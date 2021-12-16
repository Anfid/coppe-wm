mod events;
mod runner;
mod wm;
mod x11;

use crate::runner::Runner;
use crate::wm::WindowManager;
use crate::x11::X11Info;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let x11 = X11Info::init().unwrap();

    let mut wm = WindowManager::init(x11.clone(), event_tx).unwrap_or_else(|e| {
        println!("Error during wm initialization: {}", e);
        std::process::exit(1);
    });
    let mut runner = Runner::init(x11, event_rx);

    std::thread::spawn(move || runner.run());

    wm.run().unwrap();
}
