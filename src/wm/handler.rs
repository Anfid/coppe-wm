use crate::events::RunnerEvent;
use log::*;
use x11rb::connection::Connection;
use x11rb::errors::{ReplyError, ReplyOrIdError};
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::CURRENT_TIME;

use super::WindowManager;

impl WindowManager {
    pub fn handle_x_event<C>(&mut self, conn: &C, event: Event) -> Result<(), ReplyOrIdError>
    where
        C: Connection,
    {
        debug!("Got X11 event {:?}", event);
        match event {
            Event::UnmapNotify(event) => self.handle_unmap_notify(conn, event)?,
            Event::ConfigureRequest(event) => self.handle_configure_request(conn, event)?,
            Event::ConfigureNotify(event) => self.handle_configure_notify(conn, event)?,
            Event::MapRequest(event) => self.handle_map_request(conn, event)?,
            Event::Expose(event) => self.handle_expose(event)?,
            Event::EnterNotify(event) => self.handle_enter(conn, event)?,
            Event::ButtonPress(event) => self.handle_button_press(conn, event)?,
            Event::ButtonRelease(event) => self.handle_button_release(conn, event)?,
            Event::MotionNotify(event) => self.handle_motion_notify(conn, event)?,
            Event::KeyPress(event) => self.handle_key_press(event)?,
            _ => {}
        }
        Ok(())
    }

    pub fn handle_runner_event<C>(&mut self, conn: &C, event: RunnerEvent)
    where
        C: Connection,
    {
        debug!("Got runner event {:?}", event);
        match event {
            RunnerEvent::MoveWindow { .. } => {}
            _ => {}
        }
    }

    fn handle_unmap_notify<C>(
        &mut self,
        conn: &C,
        event: UnmapNotifyEvent,
    ) -> Result<(), ReplyError>
    where
        C: Connection,
    {
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

    fn handle_configure_request<C>(
        &mut self,
        conn: &C,
        event: ConfigureRequestEvent,
    ) -> Result<(), ReplyError>
    where
        C: Connection,
    {
        if let Some(_client) = self.find_window_by_id(conn, event.window) {
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
        debug!("Configure: {:?}", aux);
        conn.configure_window(event.window, &aux)?;
        Ok(())
    }

    fn handle_configure_notify<C>(
        &mut self,
        conn: &C,
        event: ConfigureNotifyEvent,
    ) -> Result<(), ReplyError>
    where
        C: Connection,
    {
        if let Some(client) = self.find_window_by_id_mut(conn, event.window) {
            client.x = event.x;
            client.y = event.y;
            client.width = event.width;
            client.height = event.height;
        }

        Ok(())
    }

    fn handle_map_request<C>(&mut self, conn: &C, event: MapRequestEvent) -> Result<(), ReplyError>
    where
        C: Connection,
    {
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
    fn handle_enter<C>(&mut self, conn: &C, event: EnterNotifyEvent) -> Result<(), ReplyError>
    where
        C: Connection,
    {
        let window = if let Some(client) = self.find_window_by_id(conn, event.event) {
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

        conn.send_event(false, window, EventMask::NO_EVENT, &event)?
            .check();

        let aux = ConfigureWindowAux::default().stack_mode(StackMode::ABOVE);
        conn.configure_window(window, &aux)?;

        conn.set_input_focus(InputFocus::PARENT, window, CURRENT_TIME)?;
        Ok(())
    }

    fn handle_button_press<C>(
        &mut self,
        conn: &C,
        event: ButtonPressEvent,
    ) -> Result<(), ReplyError>
    where
        C: Connection,
    {
        if event.detail == ButtonIndex::M1.into() {
            if let Some(client) = self.find_window_by_id(conn, event.child) {
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

    fn handle_button_release<C>(
        &mut self,
        conn: &C,
        event: ButtonReleaseEvent,
    ) -> Result<(), ReplyError>
    where
        C: Connection,
    {
        if let super::WmMode::ClientMove { .. } = self.mode {
            if event.detail == ButtonIndex::M1.into() {
                debug!("Entered Default mode");
                self.mode = super::WmMode::Default;
            }
        }

        if let Some(client) = self.find_window_by_id(conn, event.event) {
            let data = [self.wm_delete_window, 0, 0, 0, 0];
            let event = ClientMessageEvent {
                response_type: CLIENT_MESSAGE_EVENT,
                format: 32,
                sequence: 0,
                window: client.window,
                type_: self.wm_protocols,
                data: data.into(),
            };
            conn.send_event(false, client.window, EventMask::NO_EVENT, &event)?;
        }
        Ok(())
    }

    fn handle_motion_notify<C>(
        &mut self,
        conn: &C,
        event: MotionNotifyEvent,
    ) -> Result<(), ReplyError>
    where
        C: Connection,
    {
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
