use log::*;
use std::{
    collections::HashSet,
    sync::{mpsc, Arc},
    thread,
};
use x11rb::atom_manager;
use x11rb::connection::Connection;
use x11rb::errors::{ReplyError, ReplyOrIdError};
use x11rb::protocol::{xproto::*, Event};
use x11rb::x11_utils::X11Error;
use x11rb::CURRENT_TIME;

mod handler;
mod state;

use crate::client::Client;
use crate::events::{Command, WmEvent};
use crate::layout::Geometry;
use crate::X11Conn;
use state::State;

pub enum WmMode {
    Default,
    ClientMove { x: i16, y: i16, client_id: Window },
    //ClientResize,
}

atom_manager! {
    pub Atoms: AtomsCookie {
        WM_PROTOCOLS,
        WM_TAKE_FOCUS,
        WM_DELETE_WINDOW,
    }
}

pub struct WindowManager {
    pub state: State,
    pub screen_num: usize,
    pub black_gc: Gcontext,
    pub pending_expose: HashSet<Window>,
    pub atoms: Atoms,
    mode: WmMode,
    tx: mpsc::Sender<WmEvent>,
    rx: mpsc::Receiver<EventVariant>,
}

pub enum EventVariant {
    X(Event),
    Command(Command),
}

impl WindowManager {
    // TODO: Restructure
    pub fn init(
        conn: Arc<X11Conn>,
        screen_num: usize,
        tx: mpsc::Sender<WmEvent>,
        command_rx: mpsc::Receiver<Command>,
    ) -> Result<Self, ReplyOrIdError> {
        let screen = &conn.setup().roots[screen_num];
        // Try to become the window manager. This causes an error if there is already another WM.
        let change = ChangeWindowAttributesAux::default().event_mask(
            EventMask::SUBSTRUCTURE_REDIRECT
                | EventMask::SUBSTRUCTURE_NOTIFY
                | EventMask::ENTER_WINDOW,
        );

        let res = conn.change_window_attributes(screen.root, &change)?.check();
        match res {
            Err(ReplyError::X11Error(X11Error {
                error_kind: x11rb::protocol::ErrorKind::Access,
                ..
            })) => {
                error!("Another WM is already running");
                std::process::exit(1)
            }
            Err(e) => return Err(e.into()),
            _ => {
                conn.grab_button(
                    true,
                    screen.root,
                    u32::from(
                        EventMask::BUTTON_RELEASE
                            | EventMask::BUTTON_PRESS
                            | EventMask::BUTTON3_MOTION
                            | EventMask::BUTTON1_MOTION,
                    ) as u16,
                    GrabMode::ASYNC,
                    GrabMode::ASYNC,
                    screen.root,
                    0u16,
                    ButtonIndex::M1,
                    ModMask::M4,
                )?;
            }
        }

        let (event_tx, event_rx) = mpsc::channel();
        let x_event_conn = conn.clone();
        let x_tx = event_tx.clone();
        thread::spawn(move || loop {
            let mut event_opt = x_event_conn.wait_for_event().ok();
            while let Some(event) = event_opt {
                x_tx.send(EventVariant::X(event)).unwrap();
                event_opt = x_event_conn.poll_for_event().unwrap();
            }
        });
        thread::spawn(move || loop {
            match command_rx.recv() {
                Ok(event) => event_tx.send(EventVariant::Command(event)).unwrap(),
                _ => break,
            };
        });

        // TODO: Is it possible to disable autorepeat and should it be disabled?
        //let keyboard_control =
        //    ChangeKeyboardControlAux::new().auto_repeat_mode(AutoRepeatMode::OFF);
        //conn.change_keyboard_control(&keyboard_control)?;

        let atoms = Atoms::new(conn.as_ref())?;

        let black_gc = conn.generate_id()?;
        let font = conn.generate_id()?;
        // TODO: Make configurable
        conn.open_font(font, b"Fixed:size=11")?;
        let gc_aux = CreateGCAux::new()
            .graphics_exposures(0)
            .background(screen.black_pixel)
            .foreground(screen.white_pixel)
            .font(font);
        conn.create_gc(black_gc, screen.root, &gc_aux)?;
        conn.close_font(font)?;

        Ok(WindowManager {
            state: Default::default(),
            screen_num,
            black_gc,
            pending_expose: HashSet::default(),
            atoms: atoms.reply()?,
            mode: WmMode::Default,
            tx,
            rx: event_rx,
        })
    }

    pub fn run(&mut self, conn: &X11Conn) -> Result<(), ReplyError> {
        self.scan_windows(conn).unwrap();

        loop {
            self.refresh(conn).unwrap();
            conn.flush().unwrap();

            // Handle as many events as possible before refresh, then wait again
            let mut event_opt = self.rx.recv().ok();
            while let Some(event) = event_opt {
                self.handle_event(conn, event).unwrap();
                event_opt = self.rx.try_recv().ok();
            }
        }
    }

    #[allow(unused)]
    pub fn focus_client(&mut self, conn: &X11Conn, rel_idx: i32) -> Result<(), ReplyError> {
        // TODO: Implement wrapping add and sub based on amount of clients
        let idx = self.state.focused as i32 + rel_idx;
        //let idx: i32 = match direction {
        //    FocusDirection::Next => self.focused as i32 + 1,
        //    FocusDirection::Prev => self.focused as i32 - 1,
        //};

        let win = if idx >= self.state.clients.len() as i32 {
            self.state.clients[0].id
        } else if idx < 0 {
            self.state.clients[self.state.clients.len() - 1].id
        } else {
            self.state.clients[self.state.focused + 1].id
        };

        let aux = ConfigureWindowAux::default().stack_mode(StackMode::ABOVE);

        conn.configure_window(win, &aux)?;

        conn.set_input_focus(InputFocus::PARENT, win, CURRENT_TIME)?;

        Ok(())
    }

    pub fn scan_windows(&mut self, conn: &X11Conn) -> Result<(), ReplyOrIdError> {
        let screen = &conn.setup().roots[self.screen_num];
        let tree_reply = conn.query_tree(screen.root)?.reply()?;

        let mut cookies = Vec::with_capacity(tree_reply.children.len());
        for win in tree_reply.children {
            let attr = conn.get_window_attributes(win)?;
            let geom = conn.get_geometry(win)?;
            cookies.push((win, attr, geom));
        }
        for (win, attr, geom) in cookies {
            if let (Ok(attr), Ok(geom)) = (attr.reply(), geom.reply()) {
                if !attr.override_redirect && attr.map_state != MapState::UNMAPPED {
                    self.manage_window(conn, win, &geom)?;
                }
            }
        }

        Ok(())
    }

    pub fn manage_window(
        &mut self,
        conn: &X11Conn,
        win: Window,
        geom: &GetGeometryReply,
    ) -> Result<(), ReplyOrIdError> {
        info!("Managing window {:?}", win);
        let screen = &conn.setup().roots[self.screen_num];
        assert!(self.state.get_client_by_id(win).is_none());

        let geometry: Geometry = geom.into();
        let geom_reply = self.state.layout.geometry(screen.into(), geometry);

        let aux = ConfigureWindowAux::default()
            .x(i32::from(geom_reply.x))
            .y(i32::from(geom_reply.y))
            .width(u32::from(geom_reply.width))
            .height(u32::from(geom_reply.height));

        conn.configure_window(win, &aux)?;
        conn.map_window(win)?;

        self.state.clients.push(Client::new(win, geom));

        Ok(())
    }

    pub fn refresh(&mut self, _conn: &X11Conn) -> Result<(), ReplyError> {
        while let Some(&win) = self.pending_expose.iter().next() {
            self.pending_expose.remove(&win);
        }
        Ok(())
    }
}
