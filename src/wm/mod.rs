use log::*;
use x11rb::connection::Connection;
use x11rb::errors::{ReplyError, ReplyOrIdError};
use x11rb::protocol::xproto::*;
use x11rb::protocol::{Error, Event};
use x11rb::{COPY_DEPTH_FROM_PARENT, CURRENT_TIME};

use std::collections::{HashMap, HashSet};

use crate::bindings::*;
use crate::client::*;

pub type Handler = Box<dyn Fn()>;

pub struct WindowManager<'c, C: Connection> {
    pub conn: &'c C,
    pub screen_num: usize,
    pub black_gc: Gcontext,
    pub clients: Vec<Client>,
    pub pending_expose: HashSet<Window>,
    pub wm_protocols: Atom,
    pub wm_delete_window: Atom,
    key_handlers: HashMap<Key, Vec<Handler>>,
}

impl<'c, C: Connection> WindowManager<'c, C> {
    // TODO: Restructure
    pub fn init(conn: &'c C, screen_num: usize) -> Result<Self, ReplyOrIdError> {
        let screen = &conn.setup().roots[screen_num];
        // Try to become the window manager. This causes an error if there is already another WM.
        let change = ChangeWindowAttributesAux::default().event_mask(
            EventMask::SubstructureRedirect
                | EventMask::SubstructureNotify
                | EventMask::EnterWindow
                | EventMask::KeyPress
                | EventMask::ButtonPress,
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
                    (EventMask::Button3Motion | EventMask::Button1Motion) as u16,
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
        let wm_delete_window = conn.intern_atom(false, b"WM_DELETE_WINDOW")?;

        Ok(WindowManager {
            conn,
            screen_num,
            black_gc,
            clients: Vec::default(),
            pending_expose: HashSet::default(),
            wm_protocols: wm_protocols.reply()?.atom,
            wm_delete_window: wm_delete_window.reply()?.atom,
            key_handlers: HashMap::new(),
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
                EventMask::Exposure | EventMask::SubstructureNotify | EventMask::ButtonRelease,
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
            geom.width - 300,
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
            if let Some(state) = self.find_window_by_id(win) {
                if let Err(err) = self.redraw_titlebar(state) {
                    warn!(
                        "Error while redrawing window {:x?}: {:?}",
                        state.window, err
                    );
                }
            }
        }
        Ok(())
    }

    pub fn redraw_titlebar(&self, state: &Client) -> Result<(), ReplyError> {
        let close_x = state.close_x_position();
        self.conn.poly_line(
            CoordMode::Origin,
            state.frame_window,
            self.black_gc,
            &[
                Point { x: close_x, y: 0 },
                Point {
                    x: state.width as _,
                    y: 15,
                },
            ],
        )?;
        self.conn.poly_line(
            CoordMode::Origin,
            state.frame_window,
            self.black_gc,
            &[
                Point { x: close_x, y: 15 },
                Point {
                    x: state.width as _,
                    y: 0,
                },
            ],
        )?;
        let reply = self
            .conn
            .get_property(
                false,
                state.window,
                AtomEnum::WM_NAME,
                AtomEnum::STRING,
                0,
                u32::MAX,
            )?
            .reply()?;
        self.conn
            .image_text8(state.frame_window, self.black_gc, 1, 10, &reply.value)?;
        Ok(())
    }

    pub fn find_window_by_id(&self, win: Window) -> Option<&Client> {
        self.clients
            .iter()
            .find(|state| state.window == win || state.frame_window == win)
    }

    #[allow(unused)]
    pub fn find_window_by_id_mut(&mut self, win: Window) -> Option<&mut Client> {
        self.clients
            .iter_mut()
            .find(|state| state.window == win || state.frame_window == win)
    }

    pub fn handle_event(&mut self, event: Event) -> Result<(), ReplyOrIdError> {
        debug!("Got event {:?}", event);
        match event {
            Event::UnmapNotify(event) => self.handle_unmap_notify(event)?,
            Event::ConfigureRequest(event) => self.handle_configure_request(event)?,
            Event::MapRequest(event) => self.handle_map_request(event)?,
            Event::Expose(event) => self.handle_expose(event)?,
            Event::EnterNotify(event) => self.handle_enter(event)?,
            Event::MotionNotify(event) => self.handle_motion_notify(event)?,
            Event::ButtonRelease(event) => self.handle_button_release(event)?,
            Event::KeyPress(event) => self.handle_key_press(event)?,
            _ => {}
        }
        Ok(())
    }

    fn handle_unmap_notify(&mut self, event: UnmapNotifyEvent) -> Result<(), ReplyError> {
        let conn = self.conn;
        self.clients.retain(|state| {
            if state.window != event.window {
                true
            } else {
                conn.destroy_window(state.frame_window).unwrap();
                false
            }
        });
        Ok(())
    }

    fn handle_configure_request(&mut self, event: ConfigureRequestEvent) -> Result<(), ReplyError> {
        if let Some(state) = self.find_window_by_id(event.window) {
            let _ = state;
            unimplemented!()
        }
        let mut aux = ConfigureWindowAux::default();
        if event.value_mask & u16::from(ConfigWindow::X) != 0 {
            aux = aux.x(i32::from(event.x))
        }
        if event.value_mask & u16::from(ConfigWindow::Y) != 0 {
            aux = aux.y(i32::from(event.y))
        }
        if event.value_mask & u16::from(ConfigWindow::Width) != 0 {
            aux = aux.width(u32::from(event.width))
        }
        if event.value_mask & u16::from(ConfigWindow::Height) != 0 {
            aux = aux.height(u32::from(event.height))
        }
        debug!("Configure: {:?}", aux);
        self.conn.configure_window(event.window, &aux)?;
        Ok(())
    }

    fn handle_map_request(&mut self, event: MapRequestEvent) -> Result<(), ReplyError> {
        self.manage_window(
            event.window,
            &self.conn.get_geometry(event.window)?.reply()?,
        )
        .unwrap();
        Ok(())
    }

    fn handle_expose(&mut self, event: ExposeEvent) -> Result<(), ReplyError> {
        self.pending_expose.insert(event.window);
        Ok(())
    }

    fn handle_enter(&mut self, event: EnterNotifyEvent) -> Result<(), ReplyError> {
        let window = if let Some(state) = self.find_window_by_id(event.child) {
            state.window
        } else {
            event.event
        };
        self.conn
            .set_input_focus(InputFocus::Parent, window, CURRENT_TIME)?;
        Ok(())
    }

    fn handle_button_release(&mut self, event: ButtonReleaseEvent) -> Result<(), ReplyError> {
        if let Some(state) = self.find_window_by_id(event.event) {
            let data = [self.wm_delete_window, 0, 0, 0, 0];
            let event = ClientMessageEvent {
                response_type: CLIENT_MESSAGE_EVENT,
                format: 32,
                sequence: 0,
                window: state.window,
                type_: self.wm_protocols,
                data: data.into(),
            };
            self.conn
                .send_event(false, state.window, EventMask::NoEvent, &event)?;
        }
        Ok(())
    }

    fn handle_motion_notify(&mut self, event: MotionNotifyEvent) -> Result<(), ReplyError> {
        let aux = ConfigureWindowAux::default()
            .x(i32::from(event.root_x - 20))
            .y(i32::from(event.root_y - 20));

        debug!("Configure: {:?}", aux);
        self.conn.configure_window(event.child, &aux)?;

        Ok(())
    }

    fn handle_key_press(&mut self, event: KeyPressEvent) -> Result<(), ReplyError> {
        if let Some(handlers) = self.key_handlers.get(&(event.state, event.detail).into()) {
            for handler in handlers {
                handler()
            }
        }
        Ok(())
    }
}
