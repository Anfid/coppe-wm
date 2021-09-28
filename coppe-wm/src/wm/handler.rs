use log::*;
use x11rb::errors::{ReplyError, ReplyOrIdError};
use x11rb::protocol::{xproto::*, Event as XEvent};
use x11rb::CURRENT_TIME;

use super::WindowManager;
use crate::events::WmEvent;

impl WindowManager {
    pub fn handle_event(&mut self, event: XEvent) -> Result<(), ReplyOrIdError> {
        debug!("Got X11 event {:?}", event);
        WmEvent::try_from(&event).map(|e| self.tx.send(e));

        match event {
            XEvent::UnmapNotify(event) => self.handle_unmap_notify(event)?,
            XEvent::ConfigureRequest(event) => self.handle_configure_request(event)?,
            XEvent::MapRequest(event) => self.handle_map_request(event)?,
            XEvent::Expose(event) => self.handle_expose(event)?,
            XEvent::EnterNotify(event) => self.handle_enter(event)?,
            _ => {}
        }
        Ok(())
    }

    fn handle_unmap_notify(&self, event: UnmapNotifyEvent) -> Result<(), ReplyError> {
        self.x11.conn.destroy_window(event.window).unwrap();
        Ok(())
    }

    fn handle_configure_request(&self, event: ConfigureRequestEvent) -> Result<(), ReplyError> {
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
        self.x11.conn.configure_window(event.window, &aux)?;
        Ok(())
    }

    fn handle_map_request(&self, event: MapRequestEvent) -> Result<(), ReplyError> {
        self.manage_window(
            event.window,
            &self.x11.conn.get_geometry(event.window)?.reply()?,
        )
        .unwrap();
        Ok(())
    }

    fn handle_expose(&mut self, event: ExposeEvent) -> Result<(), ReplyError> {
        self.pending_expose.insert(event.window);
        Ok(())
    }

    fn handle_enter(&self, event: EnterNotifyEvent) -> Result<(), ReplyError> {
        let window = event.event;

        let data = [self.x11.atoms.WM_TAKE_FOCUS, CURRENT_TIME, 0, 0, 0];
        let event = ClientMessageEvent {
            response_type: CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window,
            type_: self.x11.atoms.WM_PROTOCOLS,
            data: data.into(),
        };

        self.x11
            .conn
            .send_event(false, window, EventMask::NO_EVENT, &event)?
            .check()?;

        let aux = ConfigureWindowAux::default().stack_mode(StackMode::ABOVE);
        self.x11.conn.configure_window(window, &aux)?;

        self.x11
            .conn
            .set_input_focus(InputFocus::PARENT, window, CURRENT_TIME)?;
        Ok(())
    }
}
