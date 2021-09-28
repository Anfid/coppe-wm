use std::sync::Arc;
use x11rb::atom_manager;
use x11rb::errors::{ConnectError, ConnectionError, ReplyError};
use x11rb::rust_connection::RustConnection as X11Conn;

atom_manager! {
    pub Atoms: AtomsCookie {
        WM_PROTOCOLS,
        WM_TAKE_FOCUS,
        WM_DELETE_WINDOW,
    }
}

#[derive(Debug, Clone)]
pub struct X11Info {
    pub conn: Arc<X11Conn>,
    pub atoms: Atoms,
    pub screen_num: usize,
}

#[derive(Debug)]
pub enum Error {
    Connect(ConnectError),
    Connection(ConnectionError),
    Reply(ReplyError),
}

impl X11Info {
    pub fn init() -> Result<Self, Error> {
        let (conn, screen_num) = X11Conn::connect(None).map_err(Error::Connect)?;
        let atoms = Atoms::new(&conn).map_err(Error::Connection)?;

        Ok(Self {
            atoms: atoms.reply().map_err(Error::Reply)?,
            conn: Arc::new(conn),
            screen_num,
        })
    }
}
