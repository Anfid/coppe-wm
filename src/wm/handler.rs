use log::*;
use x11rb::errors::{ReplyError, ReplyOrIdError};
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::CURRENT_TIME;

use super::WindowManager;
use crate::events::WMEvent;
use crate::X11Conn;

impl WindowManager {
    pub fn handle_event(&mut self, conn: &X11Conn, event: Event) -> Result<(), ReplyOrIdError> {
        debug!("Got X11 event {:?}", event);
        WMEvent::try_from(&event).map(|e| self.tx.send(e));

        match event {
            Event::UnmapNotify(event) => self.handle_unmap_notify(conn, event)?,
            Event::ConfigureRequest(event) => self.handle_configure_request(conn, event)?,
            Event::ConfigureNotify(event) => self.handle_configure_notify(event)?,
            Event::MapRequest(event) => self.handle_map_request(conn, event)?,
            Event::Expose(event) => self.handle_expose(event)?,
            Event::EnterNotify(event) => self.handle_enter(conn, event)?,
            Event::ButtonPress(event) => self.handle_button_press(event)?,
            Event::ButtonRelease(event) => self.handle_button_release(conn, event)?,
            Event::MotionNotify(event) => self.handle_motion_notify(conn, event)?,
            Event::KeyPress(event) => self.handle_key_press(event)?,
            _ => {}
        }
        Ok(())
    }

    fn handle_unmap_notify(
        &mut self,
        conn: &X11Conn,
        event: UnmapNotifyEvent,
    ) -> Result<(), ReplyError> {
        let conn = conn;
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

    fn handle_configure_request(
        &mut self,
        conn: &X11Conn,
        event: ConfigureRequestEvent,
    ) -> Result<(), ReplyError> {
        if let Some(_client) = self.find_window_by_id(event.window) {
            //
        }
        let mut aux = ConfigureWindowAux::default();
        if event.value_mask & u16::from(ConfigWindow::X) != 0 {
            aux = aux.x(i32::from(event.x))
        }
        if event.value_mask & u16::from(ConfigWindow::Y) != 0 {
            aux = aux.y(i32::from(event.y))
        }
        if event.value_mask & u16::from(ConfigWindow::WIDTH) != 0 {
            aux = aux.width(u32::from(event.width))
        }
        if event.value_mask & u16::from(ConfigWindow::HEIGHT) != 0 {
            aux = aux.height(u32::from(event.height))
        }
        debug!("Configure window: {:?}", aux);
        conn.configure_window(event.window, &aux)?;
        Ok(())
    }

    fn handle_configure_notify(&mut self, event: ConfigureNotifyEvent) -> Result<(), ReplyError> {
        if let Some(client) = self.find_window_by_id_mut(event.window) {
            client.x = event.x;
            client.y = event.y;
            client.width = event.width;
            client.height = event.height;
        }

        Ok(())
    }

    fn handle_map_request(
        &mut self,
        conn: &X11Conn,
        event: MapRequestEvent,
    ) -> Result<(), ReplyError> {
        self.manage_window(
            conn,
            event.window,
            &conn.get_geometry(event.window)?.reply()?,
        )
        .unwrap();
        Ok(())
    }

    fn handle_expose(&mut self, event: ExposeEvent) -> Result<(), ReplyError> {
        self.pending_expose.insert(event.window);
        Ok(())
    }

    // TODO: For some reason event seems to randomly return to WM
    fn handle_enter(&mut self, conn: &X11Conn, event: EnterNotifyEvent) -> Result<(), ReplyError> {
        let window = if let Some(client) = self.find_window_by_id(event.event) {
            client.window
        } else {
            event.event
        };

        let data = [self.atoms.WM_TAKE_FOCUS, CURRENT_TIME, 0, 0, 0];
        let event = ClientMessageEvent {
            response_type: CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window,
            type_: self.atoms.WM_PROTOCOLS,
            data: data.into(),
        };

        conn.send_event(false, window, EventMask::NO_EVENT, &event)?
            .check();

        let aux = ConfigureWindowAux::default().stack_mode(StackMode::ABOVE);
        conn.configure_window(window, &aux)?;

        conn.set_input_focus(InputFocus::PARENT, window, CURRENT_TIME)?;
        Ok(())
    }

    fn handle_button_press(&mut self, event: ButtonPressEvent) -> Result<(), ReplyError> {
        if event.detail == ButtonIndex::M1.into() {
            if let Some(client) = self.find_window_by_id(event.child) {
                debug!(
                    "Entered ClientMove mode with {} {}",
                    event.root_x - client.x,
                    event.root_y - client.y
                );
                self.mode = super::WmMode::ClientMove {
                    x: event.root_x - client.x,
                    y: event.root_y - client.y,
                    client_id: client.frame_window,
                };
            }
        }

        Ok(())
    }

    fn handle_button_release(
        &mut self,
        conn: &X11Conn,
        event: ButtonReleaseEvent,
    ) -> Result<(), ReplyError> {
        if let super::WmMode::ClientMove { .. } = self.mode {
            if event.detail == ButtonIndex::M1.into() {
                debug!("Entered Default mode");
                self.mode = super::WmMode::Default;
            }
        }

        if let Some(client) = self.find_window_by_id(event.event) {
            let data = [self.atoms.WM_DELETE_WINDOW, 0, 0, 0, 0];
            let event = ClientMessageEvent {
                response_type: CLIENT_MESSAGE_EVENT,
                format: 32,
                sequence: 0,
                window: client.window,
                type_: self.atoms.WM_PROTOCOLS,
                data: data.into(),
            };
            conn.send_event(false, client.window, EventMask::NO_EVENT, &event)?;
        }
        Ok(())
    }

    fn handle_motion_notify(
        &mut self,
        conn: &X11Conn,
        event: MotionNotifyEvent,
    ) -> Result<(), ReplyError> {
        match self.mode {
            super::WmMode::Default => {}
            super::WmMode::ClientMove { x, y, client_id } => {
                let aux = ConfigureWindowAux::default()
                    .x(i32::from(event.root_x - x))
                    .y(i32::from(event.root_y - y));

                debug!("Configure: {:?}", aux);
                conn.configure_window(client_id, &aux)?;
            }
        }
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
