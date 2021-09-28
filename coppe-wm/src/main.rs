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

    let mut runner = Runner::new(event_rx, x11.clone());
    std::thread::spawn(move || runner.run());

    let mut wm = WindowManager::init(x11, event_tx).unwrap();
    wm.run().unwrap();
}
