use log::*;
use x11rb::connection::Connection;
use x11rb::errors::{ReplyError, ReplyOrIdError};
use x11rb::protocol::xproto::*;
use x11rb::protocol::Error;
use x11rb::COPY_DEPTH_FROM_PARENT;

use std::collections::{HashMap, HashSet};

mod handler;

use crate::bindings::*;
use crate::client::*;

pub type Handler = Box<dyn Fn()>;

pub enum WmMode {
    Default,
    ClientMove { x: i16, y: i16, client_id: Window },
    //ClientResize,
}

pub struct WindowManager<'c, C: Connection> {
    pub conn: &'c C,
    pub screen_num: usize,
    pub black_gc: Gcontext,
    pub clients: Vec<Client>,
    pub pending_expose: HashSet<Window>,
    pub wm_protocols: Atom,
    pub wm_take_focus: Atom,
    pub wm_delete_window: Atom,
    key_handlers: HashMap<Key, Vec<Handler>>,
    mode: WmMode,
}

impl<'c, C: Connection> WindowManager<'c, C> {
    // TODO: Restructure
    pub fn init(conn: &'c C, screen_num: usize) -> Result<Self, ReplyOrIdError> {
        let screen = &conn.setup().roots[screen_num];
        // Try to become the window manager. This causes an error if there is already another WM.
        let change = ChangeWindowAttributesAux::default().event_mask(
            EventMask::SubstructureRedirect
                | EventMask::SubstructureNotify
                | EventMask::EnterWindow,
        );

        let res = conn.change_window_attributes(screen.root, &change)?.check();
        match res {
            Err(ReplyError::X11Error(Error::Access(_))) => {
                error!("Another WM is already running");
                std::process::exit(1);
            }
            _ => {
                conn.grab_button(
                    true,
                    screen.root,
                    (EventMask::ButtonRelease
                        | EventMask::ButtonPress
                        | EventMask::Button3Motion
                        | EventMask::Button1Motion) as u16,
                    GrabMode::Async,
                    GrabMode::Async,
                    screen.root,
                    0u16,
                    ButtonIndex::M1,
                    ModMask::M4,
                )?;
            }
        }

        let screen = &conn.setup().roots[screen_num];
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

        let wm_protocols = conn.intern_atom(false, b"WM_PROTOCOLS")?;
        let wm_take_focus = conn.intern_atom(false, b"WM_TAKE_FOCUS")?;
        let wm_delete_window = conn.intern_atom(false, b"WM_DELETE_WINDOW")?;

        Ok(WindowManager {
            conn,
            screen_num,
            black_gc,
            clients: Vec::default(),
            pending_expose: HashSet::default(),
            wm_protocols: wm_protocols.reply()?.atom,
            wm_take_focus: wm_take_focus.reply()?.atom,
            wm_delete_window: wm_delete_window.reply()?.atom,
            key_handlers: HashMap::new(),
            mode: WmMode::Default,
        })
    }

    pub fn run(&mut self) -> Result<(), ReplyError> {
        self.scan_windows().unwrap();

        loop {
            self.refresh().unwrap();
            self.conn.flush().unwrap();

            let mut event_opt = self.conn.wait_for_event().ok();
            while let Some(event) = event_opt {
                self.handle_event(event).unwrap();
                event_opt = self.conn.poll_for_event().unwrap();
            }
        }
    }

    pub fn bind_keys(&mut self, keys: Vec<(Key, impl Fn() + 'static)>) -> Result<(), ReplyError> {
        for (key, handler) in keys {
            self.conn.grab_key(
                true,
                self.conn.setup().roots[self.screen_num].root,
                key.modmask,
                key.keycode,
                GrabMode::Async,
                GrabMode::Async,
            )?;
            if let Some(handlers) = self.key_handlers.get_mut(&key) {
                handlers.push(Box::new(handler));
            } else {
                self.key_handlers.insert(key, vec![Box::new(handler)]);
            }
        }
        Ok(())
    }

    pub fn scan_windows(&mut self) -> Result<(), ReplyOrIdError> {
        let screen = &self.conn.setup().roots[self.screen_num];
        let tree_reply = self.conn.query_tree(screen.root)?.reply()?;

        let mut cookies = Vec::with_capacity(tree_reply.children.len());
        for win in tree_reply.children {
            let attr = self.conn.get_window_attributes(win)?;
            let geom = self.conn.get_geometry(win)?;
            cookies.push((win, attr, geom));
        }
        for (win, attr, geom) in cookies {
            if let (Ok(attr), Ok(geom)) = (attr.reply(), geom.reply()) {
                if !attr.override_redirect && attr.map_state != MapState::Unmapped {
                    self.manage_window(win, &geom)?;
                }
            }
        }

        Ok(())
    }

    pub fn manage_window(
        &mut self,
        win: Window,
        geom: &GetGeometryReply,
    ) -> Result<(), ReplyOrIdError> {
        info!("Managing window {:?}", win);
        let screen = &self.conn.setup().roots[self.screen_num];
        assert!(self.find_window_by_id(win).is_none());

        let frame_win = self.conn.generate_id()?;
        let win_aux = CreateWindowAux::new()
            .event_mask(
                EventMask::ButtonRelease
                    | EventMask::EnterWindow
                    | EventMask::PropertyChange
                    | EventMask::Exposure
                    | EventMask::SubstructureNotify,
            )
            .background_pixel(screen.black_pixel);
        // TODO: Change titlebar height calculation
        let titlebar_h = 15;
        self.conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            frame_win,
            screen.root,
            geom.x,
            geom.y,
            geom.width,
            geom.height + titlebar_h,
            1,
            WindowClass::InputOutput,
            0,
            &win_aux,
        )?;

        self.conn
            .reparent_window(win, frame_win, 0, titlebar_h as _)?;
        self.conn.map_window(win)?;
        self.conn.map_window(frame_win)?;

        self.clients.push(Client::new(win, frame_win, geom));
        Ok(())
    }

    pub fn refresh(&mut self) -> Result<(), ReplyError> {
        while let Some(&win) = self.pending_expose.iter().next() {
            self.pending_expose.remove(&win);
            if let Some(client) = self.find_window_by_id(win) {
                if let Err(err) = self.redraw_titlebar(client) {
                    warn!(
                        "Error while redrawing window {:x?}: {:?}",
                        client.window, err
                    );
                }
            }
        }
        Ok(())
    }

    pub fn redraw_titlebar(&self, client: &Client) -> Result<(), ReplyError> {
        let close_x = client.close_x_position();
        self.conn.poly_line(
            CoordMode::Origin,
            client.frame_window,
            self.black_gc,
            &[
                Point { x: close_x, y: 0 },
                Point {
                    x: client.width as _,
                    y: 15,
                },
            ],
        )?;
        self.conn.poly_line(
            CoordMode::Origin,
            client.frame_window,
            self.black_gc,
            &[
                Point { x: close_x, y: 15 },
                Point {
                    x: client.width as _,
                    y: 0,
                },
            ],
        )?;
        let reply = self
            .conn
            .get_property(
                false,
                client.window,
                AtomEnum::WM_NAME,
                AtomEnum::STRING,
                0,
                u32::MAX,
            )?
            .reply()?;
        self.conn
            .image_text8(client.frame_window, self.black_gc, 1, 10, &reply.value)?;
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
