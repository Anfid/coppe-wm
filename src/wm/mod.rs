use log::*;
use std::{
    collections::{HashMap, HashSet},
    sync::{mpsc::Sender, Arc},
};
use x11rb::atom_manager;
use x11rb::connection::Connection;
use x11rb::errors::{ReplyError, ReplyOrIdError};
use x11rb::protocol::xproto::*;
use x11rb::x11_utils::X11Error;
use x11rb::{COPY_DEPTH_FROM_PARENT, CURRENT_TIME};

mod handler;

use crate::bindings::Key;
use crate::client::Client;
use crate::events::WmEvent;
use crate::layout::Geometry;
use crate::state::State;
use crate::X11Conn;

// TODO: Change titlebar height calculation
const TITLEBAR_SIZE: u16 = 15;

pub type Handler = Box<dyn Fn()>;

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
    pub clients: Vec<Client>,
    pub pending_expose: HashSet<Window>,
    pub atoms: Atoms,
    key_handlers: HashMap<Key, Vec<Handler>>,
    mode: WmMode,
    tx: Sender<WmEvent>,
}

impl WindowManager {
    // TODO: Restructure
    pub fn init(
        conn: Arc<X11Conn>,
        screen_num: usize,
        state: State,
        tx: Sender<WmEvent>,
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
            state,
            screen_num,
            black_gc,
            clients: Vec::default(),
            pending_expose: HashSet::default(),
            atoms: atoms.reply()?,
            key_handlers: HashMap::new(),
            mode: WmMode::Default,
            tx,
        })
    }

    pub fn run(&mut self, conn: &X11Conn) -> Result<(), ReplyError> {
        self.scan_windows(conn).unwrap();

        loop {
            self.refresh(conn).unwrap();
            conn.flush().unwrap();

            // Handle as many events as possible before refresh, then wait again
            let mut event_opt = conn.wait_for_event().ok();
            while let Some(event) = event_opt {
                self.handle_event(conn, event).unwrap();
                event_opt = conn.poll_for_event().unwrap();
            }
        }
    }

    pub fn bind_keys(
        &mut self,
        conn: &X11Conn,
        keys: Vec<(Key, impl Fn() + 'static)>,
    ) -> Result<(), ReplyError> {
        let key = Key {
            modmask: ModMask::M4,
            keycode: 38,
        };
        conn.grab_key(
            true,
            conn.setup().roots[self.screen_num].root,
            key.modmask,
            key.keycode,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )?;
        for (key, handler) in keys {
            conn.grab_key(
                true,
                conn.setup().roots[self.screen_num].root,
                key.modmask,
                key.keycode,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
            )?;
            if let Some(handlers) = self.key_handlers.get_mut(&key) {
                handlers.push(Box::new(handler));
            } else {
                self.key_handlers.insert(key, vec![Box::new(handler)]);
            }
        }
        Ok(())
    }

    pub fn focus_client(&mut self, conn: &X11Conn, rel_idx: i32) -> Result<(), ReplyError> {
        // TODO: Implement wrapping add and sub based on amount of clients
        let idx = self.state.get().focused as i32 + rel_idx;
        //let idx: i32 = match direction {
        //    FocusDirection::Next => self.focused as i32 + 1,
        //    FocusDirection::Prev => self.focused as i32 - 1,
        //};
        let win = if idx >= self.clients.len() as i32 {
            self.clients[0].frame_window
        } else if idx < 0 {
            self.clients[self.clients.len() - 1].frame_window
        } else {
            self.clients[self.state.get().focused + 1].frame_window
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
        assert!(self.find_window_by_id(win).is_none());

        let mut geometry: Geometry = geom.into();
        geometry.height += TITLEBAR_SIZE;
        let geom_reply = self.state.get().layout.geometry(screen.into(), geometry);

        let frame_win = conn.generate_id()?;
        let win_aux = CreateWindowAux::new()
            .event_mask(
                EventMask::BUTTON_RELEASE
                    | EventMask::ENTER_WINDOW
                    | EventMask::PROPERTY_CHANGE
                    | EventMask::EXPOSURE
                    | EventMask::SUBSTRUCTURE_NOTIFY,
            )
            .background_pixel(screen.white_pixel);
        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            frame_win,
            screen.root,
            geom_reply.x,
            geom_reply.y,
            geom_reply.width,
            geom_reply.height,
            1,
            WindowClass::INPUT_OUTPUT,
            0,
            &win_aux,
        )?;
        let aux = ConfigureWindowAux::default()
            .x(i32::from(geom_reply.x))
            .y(i32::from(geom_reply.y))
            .width(u32::from(geom_reply.width))
            .height(u32::from(geom_reply.height - TITLEBAR_SIZE));

        conn.configure_window(win, &aux)?;
        conn.reparent_window(win, frame_win, 0, TITLEBAR_SIZE as i16)?;
        conn.map_window(win)?;
        conn.map_window(frame_win)?;

        self.clients.push(Client::new(win, frame_win, geom));
        Ok(())
    }

    pub fn refresh(&mut self, conn: &X11Conn) -> Result<(), ReplyError> {
        while let Some(&win) = self.pending_expose.iter().next() {
            self.pending_expose.remove(&win);
            if let Some(client) = self.find_window_by_id(win) {
                if let Err(err) = self.redraw_titlebar(conn, client) {
                    warn!(
                        "Error while redrawing window {:x?}: {:?}",
                        client.window, err
                    );
                }
            }
        }
        Ok(())
    }

    pub fn redraw_titlebar(&self, conn: &X11Conn, client: &Client) -> Result<(), ReplyError> {
        let close_x = client.close_x_position();
        conn.poly_line(
            CoordMode::ORIGIN,
            client.frame_window,
            self.black_gc,
            &[
                Point { x: close_x, y: 0 },
                Point {
                    x: client.width as _,
                    y: TITLEBAR_SIZE as i16,
                },
            ],
        )?;
        conn.poly_line(
            CoordMode::ORIGIN,
            client.frame_window,
            self.black_gc,
            &[
                Point {
                    x: close_x,
                    y: TITLEBAR_SIZE as i16,
                },
                Point {
                    x: client.width as _,
                    y: 0,
                },
            ],
        )?;
        let reply = conn
            .get_property(
                false,
                client.window,
                AtomEnum::WM_NAME,
                AtomEnum::STRING,
                0,
                u32::MAX,
            )?
            .reply()?;
        conn.image_text8(client.frame_window, self.black_gc, 1, 10, &reply.value)?;
        Ok(())
    }

    pub fn find_window_by_id(&self, win: Window) -> Option<&Client> {
        self.clients
            .iter()
            .find(|client| client.window == win || client.frame_window == win)
    }

    pub fn find_window_by_id_mut(&mut self, win: Window) -> Option<&mut Client> {
        self.clients
            .iter_mut()
            .find(|client| client.window == win || client.frame_window == win)
    }
}
