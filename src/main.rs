extern crate x11rb;

use std::collections::HashSet;

use x11rb::connection::Connection;
use x11rb::errors::{ReplyError, ReplyOrIdError};
use x11rb::protocol::xproto::*;
use x11rb::protocol::{Error, Event};
use x11rb::{COPY_DEPTH_FROM_PARENT, CURRENT_TIME};

/// Configuration module
pub mod config;

struct WindowState {
    window: Window,
    frame_window: Window,
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}

impl WindowState {
    fn new(window: Window, frame_window: Window, geom: &GetGeometryReply) -> WindowState {
        WindowState {
            window,
            frame_window,
            x: geom.x,
            y: geom.y,
            width: geom.width,
            height: geom.height,
        }
    }

    fn close_x_position(&self) -> i16 {
        std::cmp::max(0, self.width - 15) as _
    }
}

struct WmState<'a, C: Connection> {
    conn: &'a C,
    screen_num: usize,
    black_gc: Gcontext,
    windows: Vec<WindowState>,
    pending_expose: HashSet<Window>,
    wm_protocols: Atom,
    wm_delete_window: Atom,
}

impl<'a, C: Connection> WmState<'a, C> {
    fn new(conn: &C, screen_num: usize) -> Result<WmState<C>, ReplyOrIdError> {
        let screen = &conn.setup().roots[screen_num];
        let black_gc = conn.generate_id()?;
        let font = conn.generate_id()?;
        conn.open_font(font, config::fonts[0])?;
        let gc_aux = CreateGCAux::new()
            .graphics_exposures(0)
            .background(screen.black_pixel)
            .foreground(screen.white_pixel)
            .font(font);
        conn.create_gc(black_gc, screen.root, &gc_aux)?;
        conn.close_font(font)?;

        let wm_protocols = conn.intern_atom(false, b"WM_PROTOCOLS")?;
        let wm_delete_window = conn.intern_atom(false, b"WM_DELETE_WINDOW")?;

        Ok(WmState {
            conn,
            screen_num,
            black_gc,
            windows: Vec::default(),
            pending_expose: HashSet::default(),
            wm_protocols: wm_protocols.reply()?.atom,
            wm_delete_window: wm_delete_window.reply()?.atom,
        })
    }

    fn scan_windows(&mut self) -> Result<(), ReplyOrIdError> {
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
            } else {
                continue;
            }
        }

        Ok(())
    }

    fn manage_window(
        &mut self,
        win: Window,
        geom: &GetGeometryReply,
    ) -> Result<(), ReplyOrIdError> {
        println!("Managing window {:?}", win);
        let screen = &self.conn.setup().roots[self.screen_num];
        assert!(self.find_window_by_id(win).is_none());

        let frame_win = self.conn.generate_id()?;
        let win_aux = CreateWindowAux::new()
            .event_mask(
                EventMask::Exposure | EventMask::SubstructureNotify | EventMask::ButtonRelease,
            )
            .background_pixel(screen.white_pixel);
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

        self.windows.push(WindowState::new(win, frame_win, geom));
        Ok(())
    }

    fn refresh(&mut self) -> Result<(), ReplyError> {
        while let Some(&win) = self.pending_expose.iter().next() {
            self.pending_expose.remove(&win);
            if let Some(state) = self.find_window_by_id(win) {
                if let Err(err) = self.redraw_titlebar(state) {
                    eprintln!(
                        "Error while redrawing window {:x?}: {:?}",
                        state.window, err
                    );
                }
            }
        }
        Ok(())
    }

    fn redraw_titlebar(&self, state: &WindowState) -> Result<(), ReplyError> {
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

    fn find_window_by_id(&self, win: Window) -> Option<&WindowState> {
        self.windows
            .iter()
            .find(|state| state.window == win || state.frame_window == win)
    }

    fn find_window_by_id_mut(&mut self, win: Window) -> Option<&mut WindowState> {
        self.windows
            .iter_mut()
            .find(|state| state.window == win || state.frame_window == win)
    }

    fn handle_event(&mut self, event: Event) -> Result<(), ReplyOrIdError> {
        println!("Got event {:?}", event);
        match event {
            Event::UnmapNotify(event) => self.handle_unmap_notify(event)?,
            Event::ConfigureRequest(event) => self.handle_configure_request(event)?,
            Event::MapRequest(event) => self.handle_map_request(event)?,
            Event::Expose(event) => self.handle_expose(event)?,
            Event::EnterNotify(event) => self.handle_enter(event)?,
            Event::ButtonRelease(event) => self.handle_button_release(event)?,
            Event::KeyPress(event) => self.handle_key_press(event)?,
            _ => {}
        }
        Ok(())
    }

    fn handle_unmap_notify(&mut self, event: UnmapNotifyEvent) -> Result<(), ReplyError> {
        let conn = self.conn;
        self.windows.retain(|state| {
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
        println!("Configure: {:?}", aux);
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

    fn handle_key_press(&mut self, event: KeyPressEvent) -> Result<(), ReplyError> {
        if event.detail == 53 && event.state == 64 {
            std::process::Command::new("rofi")
                .args(&[
                    "-modi",
                    "drun,run",
                    "-show",
                    "run",
                    "-location",
                    "0",
                    "-xoffset",
                    "0",
                ])
                .spawn()
                .unwrap();
        }
        Ok(())
    }
}

fn main() {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];
    become_wm(&conn, screen).unwrap();

    let mut wm_state = WmState::new(&conn, screen_num).unwrap();
    wm_state.scan_windows().unwrap();

    std::process::Command::new("feh")
        .arg("--bg-scale")
        .arg("/home/anfid/Pictures/Wallpapers/LoneWolf.png")
        .output()
        .unwrap();

    loop {
        wm_state.refresh().unwrap();
        conn.flush().unwrap();

        let mut event_opt = conn.wait_for_event().ok();
        while let Some(event) = event_opt {
            wm_state.handle_event(event).unwrap();
            event_opt = conn.poll_for_event().unwrap();
        }
    }
}

/**
 * Checks for another WM running. If there is one, prints an error and exits.
 */
pub fn become_wm<C: Connection>(conn: &C, screen: &Screen) -> Result<(), ReplyError> {
    // Try to become the window manager. This causes an error if there is already another WM.
    let change = ChangeWindowAttributesAux::default().event_mask(
        EventMask::SubstructureRedirect
            | EventMask::SubstructureNotify
            | EventMask::EnterWindow
            | EventMask::KeyPress,
    );
    let res = conn.change_window_attributes(screen.root, &change)?.check();
    match res {
        Err(ReplyError::X11Error(Error::Access(_))) => {
            eprintln!("Another WM is already running");
            std::process::exit(1);
        }
        _ => {
            conn.grab_key(
                true,
                screen.root,
                ModMask::M4,
                53u8,
                GrabMode::Async,
                GrabMode::Async,
            )?;
            res
        }
    }
}
