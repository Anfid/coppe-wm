use log::*;
use x11rb::connection::Connection;
use x11rb::errors::{ReplyError, ReplyOrIdError};
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::CURRENT_TIME;

use super::WindowManager;

impl<'c, C: Connection> WindowManager<'c, C> {
    pub fn handle_event(&mut self, event: Event) -> Result<(), ReplyOrIdError> {
        debug!("Got event {:?}", event);
        match event {
            Event::UnmapNotify(event) => self.handle_unmap_notify(event)?,
            Event::ConfigureRequest(event) => self.handle_configure_request(event)?,
            Event::ConfigureNotify(event) => self.handle_configure_notify(event)?,
            Event::MapRequest(event) => self.handle_map_request(event)?,
            Event::Expose(event) => self.handle_expose(event)?,
            Event::EnterNotify(event) => self.handle_enter(event)?,
            Event::ButtonPress(event) => self.handle_button_press(event)?,
            Event::ButtonRelease(event) => self.handle_button_release(event)?,
            Event::MotionNotify(event) => self.handle_motion_notify(event)?,
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

    fn handle_configure_notify(&mut self, event: ConfigureNotifyEvent) -> Result<(), ReplyError> {
        if let Some(client) = self.find_window_by_id_mut(event.window) {
            client.x = event.x;
            client.y = event.y;
            client.width = event.width;
            client.height = event.height;
        }

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

    // TODO: For some reason event seems to randomly return to WM
    fn handle_enter(&mut self, event: EnterNotifyEvent) -> Result<(), ReplyError> {
        let window = if let Some(client) = self.find_window_by_id(event.event) {
            client.window
        } else {
            event.event
        };

        let data = [self.wm_take_focus, CURRENT_TIME, 0, 0, 0];
        let event = ClientMessageEvent {
            response_type: CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window,
            type_: self.wm_protocols,
            data: data.into(),
        };
        // TODO: Remove unwrap
        self.conn
            .send_event(false, window, EventMask::NoEvent, &event)?.check().unwrap();

        self.conn
            .set_input_focus(InputFocus::Parent, window, CURRENT_TIME)?;
        Ok(())
    }

    fn handle_button_press(&mut self, event: ButtonPressEvent) -> Result<(), ReplyError> {
        if event.detail == ButtonIndex::M1 as u8 {
            if let Some(client) = self.find_window_by_id(event.child) {
                debug!("Entered ClientMove mode with {} {}", event.root_x - client.x, event.root_y - client.y);
                self.mode = super::WmMode::ClientMove {
                    x: event.root_x - client.x,
                    y: event.root_y - client.y,
                    client_id: client.frame_window,
                };
            }
        }

        Ok(())
    }

    fn handle_button_release(&mut self, event: ButtonReleaseEvent) -> Result<(), ReplyError> {
        if let super::WmMode::ClientMove { .. } = self.mode {
            if event.detail == ButtonIndex::M1 as u8 {
                debug!("Entered Default mode");
                self.mode = super::WmMode::Default;
            }
        }

        if let Some(client) = self.find_window_by_id(event.event) {
            debug!("=-=-=- child: {}; client: {}", client.window, client.frame_window);
            let data = [self.wm_delete_window, 0, 0, 0, 0];
            let event = ClientMessageEvent {
                response_type: CLIENT_MESSAGE_EVENT,
                format: 32,
                sequence: 0,
                window: client.window,
                type_: self.wm_protocols,
                data: data.into(),
            };
            self.conn
                .send_event(false, client.window, EventMask::NoEvent, &event)?;
        }
        Ok(())
    }

    fn handle_motion_notify(&mut self, event: MotionNotifyEvent) -> Result<(), ReplyError> {
        match self.mode {
            super::WmMode::Default => {},
            super::WmMode::ClientMove { x, y, client_id } => {
                let aux = ConfigureWindowAux::default()
                    .x(i32::from(event.root_x - x))
                    .y(i32::from(event.root_y - y));

                debug!("Configure: {:?}", aux);
                self.conn.configure_window(client_id, &aux)?;
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
