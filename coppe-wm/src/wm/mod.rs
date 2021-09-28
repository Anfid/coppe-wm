use log::*;
use std::{collections::HashSet, sync::mpsc};
use x11rb::connection::Connection;
use x11rb::errors::{ReplyError, ReplyOrIdError};
use x11rb::protocol::xproto::*;
use x11rb::x11_utils::X11Error;

mod handler;

use crate::events::WmEvent;
use crate::x11::X11Info;

pub struct WindowManager {
    x11: X11Info,
    pub pending_expose: HashSet<Window>,
    tx: mpsc::Sender<WmEvent>,
}

impl WindowManager {
    // TODO: Restructure
    pub fn init(x11: X11Info, tx: mpsc::Sender<WmEvent>) -> Result<Self, ReplyOrIdError> {
        let screen = &x11.conn.setup().roots[x11.screen_num];
        // Try to become the window manager. This causes an error if there is already another WM.
        let change = ChangeWindowAttributesAux::default().event_mask(
            EventMask::SUBSTRUCTURE_REDIRECT
                | EventMask::SUBSTRUCTURE_NOTIFY
                | EventMask::ENTER_WINDOW,
        );

        let res = x11
            .conn
            .change_window_attributes(screen.root, &change)?
            .check();
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
                x11.conn.grab_button(
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

        Ok(WindowManager {
            x11,
            pending_expose: HashSet::default(),
            tx,
        })
    }

    pub fn run(&mut self) -> Result<(), ReplyError> {
        self.scan_windows().unwrap();

        loop {
            self.refresh().unwrap();
            self.x11.conn.flush().unwrap();

            // Handle as many events as possible before refresh, then wait again
            let mut event_opt = self.x11.conn.wait_for_event().ok();
            while let Some(event) = event_opt {
                self.handle_event(event).unwrap();
                event_opt = self.x11.conn.poll_for_event().unwrap();
            }
        }
    }

    pub fn scan_windows(&self) -> Result<(), ReplyOrIdError> {
        let screen = &self.x11.conn.setup().roots[self.x11.screen_num];
        let tree_reply = self.x11.conn.query_tree(screen.root)?.reply()?;

        let mut cookies = Vec::with_capacity(tree_reply.children.len());
        for win in tree_reply.children {
            let attr = self.x11.conn.get_window_attributes(win)?;
            let geom = self.x11.conn.get_geometry(win)?;
            cookies.push((win, attr, geom));
        }
        for (win, attr, geom) in cookies {
            if let (Ok(attr), Ok(geom)) = (attr.reply(), geom.reply()) {
                if !attr.override_redirect && attr.map_state != MapState::UNMAPPED {
                    self.manage_window(win, &geom)?;
                }
            }
        }

        Ok(())
    }

    pub fn manage_window(
        &self,
        win: Window,
        geom: &GetGeometryReply,
    ) -> Result<(), ReplyOrIdError> {
        info!("Managing window {:?}", win);

        let aux = ConfigureWindowAux::default()
            .x(i32::from(geom.x))
            .y(i32::from(geom.y))
            .width(u32::from(geom.width))
            .height(u32::from(geom.height));

        self.x11.conn.configure_window(win, &aux)?;
        self.x11.conn.map_window(win)?;

        Ok(())
    }

    pub fn refresh(&mut self) -> Result<(), ReplyError> {
        while let Some(&win) = self.pending_expose.iter().next() {
            self.pending_expose.remove(&win);
        }
        Ok(())
    }
}
