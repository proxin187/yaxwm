use crate::config::{Config, Padding};
use crate::log::{self, Severity};
use crate::server::Server;
use crate::startup;

use yaxi::display::{self, Display, Atom, TryClone};
use yaxi::proto::{Event, EventMask, RevertTo, WindowClass};
use yaxi::window::{Window, WindowKind, WindowArguments, ValuesBuilder, PropFormat, PropMode};

use std::os::unix::net::UnixStream;

use proto::Request;


pub struct Client {
    window: Window<UnixStream>,
    float: bool,
}

impl Client {
    pub fn new(window: Window<UnixStream>, float: bool) -> Client {
        Client {
            window,
            float,
        }
    }
}

pub struct Workspaces {
    workspaces: Vec<Vec<Client>>,
    current: usize,
}

impl Workspaces {
    pub fn new(count: usize) -> Workspaces {
        let mut workspaces: Vec<Vec<Client>> = Vec::new();

        workspaces.resize_with(count, Vec::new);

        Workspaces {
            workspaces,
            current: 0,
        }
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
        for (w_idx, workspace) in self.workspaces.iter_mut().enumerate() {
            if w_idx == self.current {
                let windows = workspace.len();

                for (index, client) in workspace.iter_mut().enumerate() {
                    if !client.float {
                        let win = (index + 1 < windows).then(|| area.split()).unwrap_or(area);

                        client.window.mov_resize(win.x + gaps, win.y + gaps, win.width - (gaps * 2), win.height - (gaps * 2))?;
                    }

                    client.window.map(WindowKind::Window)?;
                }
            } else {
                for client in workspace {
                    client.window.unmap(WindowKind::Window)?;
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
    root: Window<UnixStream>,
}

impl Monitors {
    pub fn new(monitors: Vec<Monitor>, root: Window<UnixStream>) -> Monitors {
        Monitors {
            monitors,
            root,
        }
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

pub struct WindowManager {
    display: Display<UnixStream>,
    root: Window<UnixStream>,
    monitors: Monitors,
    server: Server,
    config: Config,
    should_close: bool,
}

impl WindowManager {
    pub fn new() -> Result<WindowManager, Box<dyn std::error::Error>> {
        let display = display::open_unix(1)?;
        let root = display.default_root_window()?;

        Ok(WindowManager {
            display,
            root: *root.try_clone()?,
            monitors: Monitors::new(vec![Monitor {
                area: Area::new(0, 0, 800, 600),
                workspace: Workspaces::new(4),
            }], root),
            server: Server::new(),
            config: Config::default(),
            should_close: false,
        })
    }

    fn setup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.root.select_input(&[EventMask::SubstructureNotify, EventMask::SubstructureRedirect, EventMask::EnterWindow, EventMask::FocusChange])?;

        self.server.listen()?;

        startup::startup()?;

        self.set_supporting_ewmh()?;

        Ok(())
    }

    fn set_supporting_ewmh(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let net_wm_check = self.display.intern_atom("_NET_SUPPORTING_WM_CHECK", false)?;
        let net_wm_name = self.display.intern_atom("_NET_WM_NAME", false)?;
        let utf8_string = self.display.intern_atom("UTF8_STRING", false)?;

        let mut window = self.root.create_window(WindowArguments {
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

        window.change_property(net_wm_check, Atom::WINDOW, PropFormat::Format32, PropMode::Replace, &window.id().to_le_bytes())?;

        window.change_property(net_wm_name, utf8_string, PropFormat::Format8, PropMode::Replace, b"yaxwm")?;

        self.root.change_property(net_wm_check, Atom::WINDOW, PropFormat::Format32, PropMode::Replace, &window.id().to_le_bytes())?;

        Ok(())
    }

    fn tile(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.monitors.all(|monitor| {
            monitor.workspace.tile(monitor.area.pad(self.config.padding), self.config.windows.gaps)
        })?;

        Ok(())
    }

    /*
    fn focused_win<F>(&mut self, f: F) -> Result<(), Box<dyn std::error::Error>> where F: Fn(Window<UnixStream>) -> Result<(), Box<dyn std::error::Error>> {
        let focus = self.display.get_input_focus()?;

        if focus.window != self.root.id() {
            f(self.display.window_from_id(focus.window)?)
        } else {
            Ok(())
        }
    }
    */

    fn focused_client<F>(&mut self, f: F) -> Result<(), Box<dyn std::error::Error>> where F: Fn(&mut Client) -> Result<(), Box<dyn std::error::Error>> {
        let focus = self.display.get_input_focus()?;

        self.monitors.focused(|monitor| {
            if let Some(index) = monitor.workspace.find(focus.window) {
                f(&mut monitor.workspace.workspaces[monitor.workspace.current][index])?;
            }

            Ok(())
        })
    }

    fn set_focused_border(&mut self, focused: u32) -> Result<(), Box<dyn std::error::Error>> {
        if focused != self.root.id() {
            let borders = self.config.windows.borders;

            self.monitors.focused(|monitor| {
                monitor.workspace.map_clients(|client| {
                    client.window.set_border_width(borders.width)?;

                    client.window.set_border_pixel(borders.normal)?;

                    Ok(())
                })?;

                Ok(())
            })?;

            self.display.window_from_id(focused)?.set_border_pixel(borders.focused)?;
        }

        Ok(())
    }

    fn handle_incoming(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: we want to auto retile when the config is updated

        for sequence in self.server.incoming()? {
            println!("sequence: {:?}", sequence);
            match sequence.request {
                Request::Workspace => {
                    self.monitors.focused(|monitor| {
                        monitor.workspace.current = sequence.value.max(1) as usize - 1;

                        monitor.workspace.tile(monitor.area.pad(self.config.padding), self.config.windows.gaps)
                    })?;
                },
                Request::Kill => {
                    self.focused_client(|client| client.window.kill())?;
                },
                Request::Close => {
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
                Request::PaddingTop => self.config.padding.top = sequence.value as u16,
                Request::PaddingBottom => self.config.padding.bottom = sequence.value as u16,
                Request::PaddingLeft => self.config.padding.left = sequence.value as u16,
                Request::PaddingRight => self.config.padding.right = sequence.value as u16,
                Request::WindowGaps => self.config.windows.gaps = sequence.value as u16,
                Request::FocusedBorder => self.config.windows.borders.focused = sequence.value,
                Request::NormalBorder => self.config.windows.borders.normal = sequence.value,
                Request::BorderWidth => self.config.windows.borders.width = sequence.value as u16,
                Request::FloatToggle => {
                    self.focused_client(|client| {
                        client.float = !client.float;

                        Ok(())
                    })?;
                },
                Request::FloatRight => {
                    // TODO: we need get window attributes
                    /*
                    self.focused_client(|client| {
                        if client.float {
                            client.window.mov()?;
                        }

                        Ok((())
                    })?;
                    */
                },
                Request::FloatLeft => {},
                Request::FloatUp => {},
                Request::FloatDown => {},
                Request::Unknown => {},
            }
        }

        Ok(())
    }

    fn handle_event(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.display.next_event()? {
            Event::MapRequest { window, .. } => {
                log::write(format!("map request: {}\n", window), Severity::Info)?;

                self.monitors.focused(|monitor| {
                    monitor.workspace.insert(Client::new(self.display.window_from_id(window)?, false));

                    Ok(())
                })?;

                self.tile()?;

                let mut window = self.display.window_from_id(window)?;

                window.select_input(&[EventMask::SubstructureNotify, EventMask::SubstructureRedirect, EventMask::EnterWindow, EventMask::FocusChange])?;

                window.set_input_focus(RevertTo::Parent)?;

                self.set_focused_border(window.id())?;
            },
            Event::UnmapNotify { window, .. } => {
                log::write(format!("unmap notify: {}\n", window), Severity::Info)?;

                self.monitors.all(|monitor| {
                    if let Some(index) = monitor.workspace.find(window) {
                        monitor.workspace.remove(index);

                        monitor.workspace.change_focus(window, |index| index - 1)?;
                    }

                    Ok(())
                })?;

                self.tile()?;
            },
            Event::EnterNotify { window, .. } => {
                log::write(format!("enter notify: {}\n", window), Severity::Info)?;

                if window != self.root.id() {
                    self.display.window_from_id(window)?.set_input_focus(RevertTo::Parent)?;
                }
            },
            Event::FocusIn { window, .. } => {
                self.set_focused_border(window)?;
            },
            Event::ConfigureRequest { stack_mode, parent, window, sibling, x, y, width, height, border_width, mask } => {
                // TODO: looks like there is something wrong with configure request,
                //
                // maybe xterm waits for a configure notify after it sends the configure request?

                log::write(format!("configure request: {}\n", window), Severity::Info)?;

                /*
                let mut window = self.display.window_from_id(window)?;

                window.configure();
                */
            },
            _ => {},
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.setup()?;

        log::write("yaxum is running\n", Severity::Info)?;

        while !self.should_close {
            if self.display.poll_event()? {
                self.handle_event()?;
            }

            self.handle_incoming()?;
        }

        Ok(())
    }
}


