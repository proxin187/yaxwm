use crate::config::{Config, Padding};
use crate::log::{self, Severity};
use crate::server::Server;
use crate::startup;

use yaxi::display::request::GetGeometryResponse;
use yaxi::display::{self, Display, Atom};
use yaxi::proto::{Event, EventMask, EventKind, KeyMask, Button, Cursor, RevertTo, WindowClass, PointerMode, KeyboardMode, ClientMessageData};
use yaxi::window::{Window, WindowKind, WindowArguments, ValuesBuilder};
use yaxi::ewmh::EwmhWindowType;

use proto::Request;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Tiled,
    Float,
    Dock,
}

impl From<&[EwmhWindowType]> for State {
    fn from(type_: &[EwmhWindowType]) -> State {
        if type_.contains(&EwmhWindowType::Dock) {
            State::Dock
        } else if type_.contains(&EwmhWindowType::Splash) || type_.contains(&EwmhWindowType::Utility) || type_.contains(&EwmhWindowType::Dialog) {
            State::Float
        } else {
            State::Tiled
        }
    }
}

pub struct Client {
    window: Window,
    state: State,
}

impl Client {
    pub fn new(window: Window, state: State) -> Client {
        Client {
            window,
            state,
        }
    }
}

pub struct Workspaces {
    workspaces: Vec<Vec<Client>>,
    current: usize,
}

impl Workspaces {
    pub fn new() -> Workspaces {
        Workspaces {
            workspaces: Vec::new(),
            current: 0,
        }
    }

    pub fn resize(&mut self, size: usize) {
        if size >= self.len() {
            self.workspaces.resize_with(size, Vec::new);
        } else if size > 0 {
            let excess = self.workspaces.drain(size..self.len()).flatten().collect::<Vec<Client>>();

            self.workspaces[size - 1].extend(excess);

            self.workspaces.truncate(self.len() - size);
        }
    }

    pub fn len(&self) -> usize {
        self.workspaces.len()
    }

    pub fn insert(&mut self, client: Client) {
        self.workspaces[self.current].push(client);
    }

    pub fn remove(&mut self, index: usize) {
        self.workspaces[self.current].remove(index);
    }

    pub fn find(&self, wid: u32) -> Option<usize> {
        self.workspaces[self.current].iter().position(|client| client.window.id() == wid)
    }

    pub fn is_float(&self, wid: u32) -> bool {
        match self.find(wid) {
            Some(index) => self.workspaces[self.current][index].state == State::Float,
            None => false,
        }
    }

    pub fn change_focus<F>(&mut self, wid: u32, f: F) -> Result<(), Box<dyn std::error::Error>> where F: Fn(usize) -> usize {
        if let Some(client) = self.find(wid).and_then(|index| self.workspaces[self.current].get_mut(f(index))) {
            client.window.set_input_focus(RevertTo::Parent)?;
        }

        Ok(())
    }

    pub fn map_clients<F>(&mut self, f: F) -> Result<(), Box<dyn std::error::Error>> where F: Fn(&mut Client) -> Result<(), Box<dyn std::error::Error>> {
        for workspace in self.workspaces.iter_mut() {
            for client in workspace {
                f(client)?;
            }
        }

        Ok(())
    }

    pub fn tile(&mut self, mut area: Area, gaps: u16) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(workspace) = self.workspaces.get_mut(self.current) {
            let ignored = workspace.iter().map(|client| client.state != State::Tiled).collect::<Vec<bool>>();

            for (index, client) in workspace.iter_mut().enumerate() {
                match client.state {
                    State::Tiled => {
                        let tiled_clients_left = ignored[index + 1..].iter().filter(|ignore| !**ignore).count();

                        let win = (tiled_clients_left > 0).then(|| area.split()).unwrap_or(area);

                        client.window.mov_resize(win.x + gaps, win.y + gaps, win.width - (gaps * 2), win.height - (gaps * 2))?;
                    },
                    _ => {},
                }

                client.window.map(WindowKind::Window)?;
            }
        }

        for (w_idx, workspace) in self.workspaces.iter_mut().enumerate() {
            if w_idx != self.current {
                for client in workspace {
                    match client.state {
                        State::Tiled | State::Float => {
                            client.window.unmap(WindowKind::Window)?;
                        },
                        _ => {},
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Area {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl Area {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Area {
        Area {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, x: u16, y: u16) -> bool {
        (x >= self.x && self.y >= self.y) && (self.x + self.width > x && self.y + self.height > y)
    }

    pub fn pad(&self, padding: Padding) -> Area {
        Area {
            x: self.x + padding.left,
            y: self.y + padding.top,
            width: self.width - padding.right - padding.left,
            height: self.height - padding.bottom - padding.top,
        }
    }

    pub fn split(&mut self) -> Area {
        let area = self.clone();

        if self.width > self.height {
            *self = Area::new(area.x + (area.width / 2), area.y, area.width / 2, area.height);

            Area::new(area.x, area.y, area.width / 2, area.height)
        } else {
            *self = Area::new(area.x, area.y + (area.height / 2), area.width, area.height / 2);

            Area::new(area.x, area.y, area.width, area.height / 2)
        }
    }
}

pub struct Monitor {
    area: Area,
    workspace: Workspaces,
}

pub struct Monitors {
    monitors: Vec<Monitor>,
    root: Window,
}

impl Monitors {
    pub fn new(root: Window) -> Monitors {
        Monitors {
            monitors: Vec::new(),
            root,
        }
    }

    pub fn append(&mut self, monitor: Monitor) {
        self.monitors.push(monitor);
    }

    pub fn is_tiled(&mut self, wid: u32) -> bool {
        self.monitors.iter()
            .map(|monitor| monitor.workspace.is_float(wid))
            .any(|float| !float)
    }

    pub fn focused<F>(&mut self, f: F) -> Result<(), Box<dyn std::error::Error>> where F: Fn(&mut Monitor) -> Result<(), Box<dyn std::error::Error>> {
        let pointer = self.root.query_pointer()?;

        for monitor in &mut self.monitors {
            if monitor.area.contains(pointer.root_x, pointer.root_y) {
                f(monitor)?;
            }
        }

        Ok(())
    }

    pub fn all<F>(&mut self, f: F) -> Result<(), Box<dyn std::error::Error>> where F: Fn(&mut Monitor) -> Result<(), Box<dyn std::error::Error>> {
        for monitor in &mut self.monitors {
            f(monitor)?;
        }

        Ok(())
    }
}

pub struct Grab {
    button: Button,
    window: Window,
    geometry: GetGeometryResponse,
    x: u16,
    y: u16,
}

impl Grab {
    pub fn new(button: Button, window: Window, geometry: GetGeometryResponse, x: u16, y: u16) -> Grab {
        Grab {
            button,
            window,
            geometry,
            x,
            y,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Atoms {
    wm_delete: Atom,
    wm_protocols: Atom,
}

pub struct WindowManager {
    display: Display,
    root: Window,
    monitors: Monitors,
    server: Server,
    config: Config,
    atoms: Atoms,
    grab: Option<Grab>,
    should_close: bool,
}

impl WindowManager {
    pub fn new() -> Result<WindowManager, Box<dyn std::error::Error>> {
        let display = display::open(Some(":2"))?;
        let root = display.default_root_window()?;

        let atoms = Atoms {
            wm_delete: display.intern_atom("WM_DELETE_WINDOW", false)?,
            wm_protocols: display.intern_atom("WM_PROTOCOLS", false)?,
        };

        Ok(WindowManager {
            display,
            root: root.clone(),
            monitors: Monitors::new(root),
            server: Server::new(),
            config: Config::default(),
            atoms,
            grab: None,
            should_close: false,
        })
    }

    fn setup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.root.select_input(&[
            EventMask::SubstructureNotify,
            EventMask::SubstructureRedirect,
            EventMask::EnterWindow,
            EventMask::FocusChange,
        ])?;

        for button in [Button::Button1, Button::Button3] {
            self.root.grab_button(
                button,
                vec![KeyMask::Mod4],
                vec![EventMask::ButtonPress, EventMask::ButtonRelease, EventMask::ButtonMotion],
                Cursor::Nop,
                PointerMode::Asynchronous,
                KeyboardMode::Asynchronous,
                true,
                0,
            )?;
        }

        self.server.listen()?;

        self.set_supporting_ewmh()?;

        self.load_monitors()?;

        startup::startup()?;

        Ok(())
    }

    fn load_monitors(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let xinerama = self.display.query_xinerama()?;

        for screen in xinerama.query_screens()? {
            self.monitors.append(Monitor {
                area: Area::new(screen.x, screen.y, screen.width, screen.height),
                workspace: Workspaces::new(),
            });
        }

        Ok(())
    }

    fn set_supporting_ewmh(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        /*
        let net_wm_check = self.display.intern_atom("_NET_SUPPORTING_WM_CHECK", false)?;
        let net_wm_name = self.display.intern_atom("_NET_WM_NAME", false)?;
        let utf8_string = self.display.intern_atom("UTF8_STRING", false)?;
        */

        /*
        window.change_property(net_wm_check, Atom::WINDOW, PropFormat::Format32, PropMode::Replace, &window.id().to_le_bytes())?;

        window.change_property(net_wm_name, utf8_string, PropFormat::Format8, PropMode::Replace, b"yaxiwm")?;

        self.root.change_property(net_wm_check, Atom::WINDOW, PropFormat::Format32, PropMode::Replace, &window.id().to_le_bytes())?;
        */

        let window = self.root.create_window(WindowArguments {
            depth: self.root.depth(),
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            class: WindowClass::InputOutput,
            border_width: 0,
            visual: self.root.visual(),
            values: ValuesBuilder::new(vec![]),
        })?;

        window.ewmh_set_supporting_wm_check(window.id())?;

        window.ewmh_set_wm_name("yaxi")?;

        self.root.ewmh_set_supporting_wm_check(window.id())?;

        Ok(())
    }

    fn tile(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.monitors.all(|monitor| {
            monitor.workspace.tile(monitor.area.pad(self.config.padding), self.config.windows.gaps)
        })?;

        Ok(())
    }

    fn focused_client<F>(&mut self, f: F) -> Result<(), Box<dyn std::error::Error>> where F: Fn(&mut Client) -> Result<(), Box<dyn std::error::Error>> {
        let focus = self.display.get_input_focus()?;

        self.monitors.focused(|monitor| {
            if let Some(index) = monitor.workspace.find(focus.window) {
                f(&mut monitor.workspace.workspaces[monitor.workspace.current][index])?;
            }

            Ok(())
        })
    }

    /*
    // TODO: finish this, no question, DONT BE LAZY!
    fn move_window_monitor(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let focus = self.display.get_input_focus()?;

        Ok(())
    }
    */

    fn mov_resize_focused<F>(&mut self, transform: F) -> Result<(), Box<dyn std::error::Error>> where F: Fn(u16, u16, u16, u16) -> (u16, u16, u16, u16) {
        self.focused_client(|client| {
            if client.state == State::Float {
                let geometry = client.window.get_geometry()?;

                let (x, y, width, height) = transform(geometry.x, geometry.y, geometry.width, geometry.height);

                client.window.mov_resize(x, y, width, height)?;
            }

            Ok(())
        })
    }

    fn update_borders(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let focus = self.display.get_input_focus()?;

        if focus.window != self.root.id() && focus.window > 1 {
            let window = self.display.window_from_id(focus.window)?;

            self.set_border(&window)?;
        }

        Ok(())
    }

    fn set_border(&mut self, window: &Window) -> Result<(), Box<dyn std::error::Error>> {
        if !window.ewmh_get_wm_window_type()?.contains(&EwmhWindowType::Dock) {
            let borders = self.config.windows.borders;

            self.monitors.focused(|monitor| {
                monitor.workspace.map_clients(|client| {
                    client.window.set_border_width(borders.width)?;

                    client.window.set_border_pixel(borders.normal).map_err(|err| err.into())
                })
            })?;

            window.set_border_pixel(borders.focused)?;
        }

        Ok(())
    }

    fn handle_incoming(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for sequence in self.server.incoming()? {
            println!("sequence: {:?}", sequence);

            match sequence.request {
                Request::Workspace => {
                    self.monitors.focused(|monitor| {
                        if sequence.value.max(1) - 1 < monitor.workspace.len() as u32 {
                            monitor.workspace.current = sequence.value.max(1) as usize - 1;
                        }

                        monitor.workspace.tile(monitor.area.pad(self.config.padding), self.config.windows.gaps)
                    })?;
                },
                Request::Kill => {
                    self.focused_client(|client| client.window.kill().map_err(|err| err.into()))?;
                },
                Request::Close => {
                    let atoms = self.atoms.clone();

                    self.focused_client(|client| {
                        client.window.send_event(Event::ClientMessage {
                            format: 32,
                            window: client.window.id(),
                            type_: atoms.wm_protocols,
                            data: ClientMessageData::Long([atoms.wm_delete.id(), 0, 0, 0, 0]),
                        }, vec![], false).map_err(|err| err.into())
                    })?;
                },
                Request::FocusUp | Request::FocusDown | Request::FocusMaster => {
                    let focus = self.display.get_input_focus()?;

                    self.monitors.focused(|monitor| {
                        match sequence.request {
                            Request::FocusUp => monitor.workspace.change_focus(focus.window, |index| index.max(1) - 1),
                            Request::FocusDown => monitor.workspace.change_focus(focus.window, |index| index + 1),
                            Request::FocusMaster => monitor.workspace.change_focus(focus.window, |_| 0),
                            _ => Ok(()),
                        }
                    })?;
                },
                Request::PaddingTop | Request::PaddingBottom | Request::PaddingLeft | Request::PaddingRight | Request::WindowGaps => {
                    match sequence.request {
                        Request::PaddingTop => self.config.padding.top = sequence.value as u16,
                        Request::PaddingBottom => self.config.padding.bottom = sequence.value as u16,
                        Request::PaddingLeft => self.config.padding.left = sequence.value as u16,
                        Request::PaddingRight => self.config.padding.right = sequence.value as u16,
                        Request::WindowGaps => self.config.windows.gaps = sequence.value as u16,
                        _ => unreachable!(),
                    }

                    self.tile()?;
                },
                Request::FocusedBorder | Request::NormalBorder | Request::BorderWidth => {
                    match sequence.request {
                        Request::FocusedBorder => self.config.windows.borders.focused = sequence.value,
                        Request::NormalBorder => self.config.windows.borders.normal = sequence.value,
                        Request::BorderWidth => self.config.windows.borders.width = sequence.value as u16,
                        _ => unreachable!(),
                    }

                    self.update_borders()?;
                },
                Request::FloatToggle => {
                    self.focused_client(|client| {
                        if client.state == State::Float {
                            client.state = State::Tiled;
                        } else if client.state != State::Dock {
                            client.state = State::Float;
                        }

                        Ok(())
                    })?;

                    self.tile()?;
                },
                Request::FloatRight => self.mov_resize_focused(|x, y, width, height| (x + sequence.value as u16, y, width, height))?,
                Request::FloatLeft => self.mov_resize_focused(|x, y, width, height| (x - (sequence.value as u16).min(x), y, width, height))?,
                Request::FloatUp => self.mov_resize_focused(|x, y, width, height| (x, y - (sequence.value as u16).min(y), width, height))?,
                Request::FloatDown => self.mov_resize_focused(|x, y, width, height| (x, y + sequence.value as u16, width, height))?,
                Request::ResizeRight => self.mov_resize_focused(|x, y, width, height| (x, y, width + sequence.value as u16, height))?,
                Request::ResizeLeft => self.mov_resize_focused(|x, y, width, height| (x, y, width - (sequence.value as u16).min(width), height))?,
                Request::ResizeUp => self.mov_resize_focused(|x, y, width, height| (x, y, width, height - (sequence.value as u16).min(height)))?,
                Request::ResizeDown => self.mov_resize_focused(|x, y, width, height| (x, y, width, height + sequence.value as u16))?,
                Request::EnableMouse => self.config.windows.mouse_movement = true,
                Request::DisableMouse => self.config.windows.mouse_movement = false,
                Request::WorkspacePerMonitor => {
                    self.monitors.all(|monitor| {
                        monitor.workspace.resize(sequence.value as usize);

                        Ok(())
                    })?;
                },
                Request::MonitorNext => {
                },
                Request::MonitorPrevious => {
                },
                Request::Unknown => {},
            }
        }

        Ok(())
    }

    fn handle_event(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.display.next_event()? {
            Event::MapRequest { window, .. } => {
                log::write(format!("map request: {}\n", window), Severity::Info)?;

                let window = self.display.window_from_id(window)?;
                let type_ = window.ewmh_get_wm_window_type()?;

                window.select_input(&[EventMask::SubstructureNotify, EventMask::SubstructureRedirect, EventMask::EnterWindow, EventMask::FocusChange])?;

                window.map(WindowKind::Window)?;

                // TODO: this fails when we launch rmenu, its a error within
                // ewmh_get_wm_window_type, specificaly the get_property reads wrong length

                if !type_.contains(&EwmhWindowType::Dock) {
                    window.set_input_focus(RevertTo::Parent)?;

                    self.set_border(&window)?;

                    self.monitors.focused(move |monitor| {
                        if monitor.workspace.find(window.id()).is_none() {
                            monitor.workspace.insert(Client::new(window.clone(), State::from(type_.as_slice())));
                        }

                        Ok(())
                    })?;

                    self.tile()?;
                }
            },
            Event::UnmapNotify { window, .. } => {
                log::write(format!("unmap notify: {}\n", window), Severity::Info)?;

                self.monitors.all(|monitor| {
                    if let Some(index) = monitor.workspace.find(window) {
                        println!("removing: {}", index);

                        monitor.workspace.remove(index);

                        // TODO: maybe we dont need to automatically change focus
                        // monitor.workspace.change_focus(window, |index| index - 1)?;
                    }

                    Ok(())
                })?;

                self.tile()?;
            },
            Event::EnterNotify { window, .. } => {
                log::write(format!("enter notify: {}\n", window), Severity::Info)?;

                if window != self.root.id() && window > 1 {
                    let window = self.display.window_from_id(window)?;

                    if !window.ewmh_get_wm_window_type()?.contains(&EwmhWindowType::Dock) {
                        window.set_input_focus(RevertTo::Parent)?;
                    }
                }
            },
            Event::FocusIn { window, .. } => {
                log::write(format!("focus in: {}\n", window), Severity::Info)?;

                if window != self.root.id() && window > 1 {
                    let window = self.display.window_from_id(window)?;

                    if !window.ewmh_get_wm_window_type()?.contains(&EwmhWindowType::Dock) {
                        self.set_border(&window)?;
                    }
                }
            },
            Event::ButtonEvent { kind, coordinates, subwindow, button, .. } => match kind {
                EventKind::Press => {
                    if !self.monitors.is_tiled(subwindow) && self.config.windows.mouse_movement {
                        let window = self.display.window_from_id(subwindow)?;

                        window.raise()?;

                        window.grab_pointer(
                            vec![EventMask::PointerMotion, EventMask::ButtonRelease],
                            Cursor::Nop,
                            PointerMode::Asynchronous,
                            KeyboardMode::Asynchronous,
                            true,
                            0,
                        )?;

                        let geometry = window.get_geometry()?;

                        self.grab.replace(Grab::new(button, window, geometry, coordinates.root_x, coordinates.root_y));
                    }
                },
                EventKind::Release => {
                    if self.grab.is_some() {
                        self.display.ungrab_pointer()?;

                        self.grab = None;
                    }
                },
            },
            Event::MotionNotify { coordinates, .. } => {
                log::write(format!("motion notify: {:?}\n", coordinates), Severity::Info)?;

                if let Some(grab) = &mut self.grab {
                    let x_diff = coordinates.root_x as i16 - grab.x as i16;
                    let y_diff = coordinates.root_y as i16 - grab.y as i16;

                    match grab.button {
                        Button::Button1 => {
                            grab.window.mov((grab.geometry.x as i16 + x_diff) as u16, (grab.geometry.y as i16 + y_diff) as u16)?;
                        },
                        Button::Button3 => {
                            grab.window.resize((grab.geometry.width as i16 + x_diff) as u16, (grab.geometry.height as i16 + y_diff) as u16)?;
                        },
                        _ => {},
                    }
                }
            },
            Event::ConfigureRequest { window, values } => {
                log::write(format!("configure request: {}, values: {:?}\n", window, values), Severity::Info)?;

                let window = self.display.window_from_id(window)?;

                if window.ewmh_get_wm_window_type()?.contains(&EwmhWindowType::Dock) {
                    window.configure(ValuesBuilder::new(values))?;
                }
            },
            _ => {},
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.setup()?;

        // TODO: there is a bug when running rmenu before opening any other applications

        log::write("yaxum is running\n", Severity::Info)?;

        while !self.should_close {
            // TODO: THE ERROR ORIGINATES FROM POLL_EVENT, MEANING POLL_EVENT RETURNS IT
            // the error is most likely caused from something inside handle_incoming

            if self.display.poll_event()? {
                self.handle_event()?;
            }

            self.handle_incoming()?;
        }

        Ok(())
    }
}


