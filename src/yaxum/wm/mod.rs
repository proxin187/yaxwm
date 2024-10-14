use crate::config::{Config, Padding};
use crate::log::{self, Severity};
use crate::server::Server;
use crate::startup;

use yaxi::display::{self, Display, TryClone};
use yaxi::proto::{Event, EventMask, RevertTo};
use yaxi::window::{Window, WindowKind};

use std::os::unix::net::UnixStream;

use proto::Request;


pub struct Workspaces {
    workspaces: Vec<Vec<Window<UnixStream>>>,
    current: usize,
}

impl Workspaces {
    pub fn new(count: usize) -> Workspaces {
        let mut workspaces: Vec<Vec<Window<UnixStream>>> = Vec::new();

        workspaces.resize_with(count, Vec::new);

        Workspaces {
            workspaces,
            current: 0,
        }
    }

    pub fn insert(&mut self, window: Window<UnixStream>) {
        self.workspaces[self.current].push(window);
    }

    pub fn remove(&mut self, index: usize) {
        self.workspaces[self.current].remove(index);
    }

    pub fn find(&mut self, wid: u32) -> Option<usize> {
        self.workspaces[self.current].iter().position(|window| window.id() == wid)
    }

    pub fn set_nearest_input_focus(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        let index = index.min(self.workspaces[self.current].len().max(1) - 1);

        if let Some(window) = self.workspaces[self.current].get_mut(index) {
            window.set_input_focus(RevertTo::Parent)?;
        }

        Ok(())
    }

    pub fn tile(&mut self, mut area: Area) -> Result<(), Box<dyn std::error::Error>> {
        for (w_idx, workspace) in self.workspaces.iter_mut().enumerate() {
            if w_idx == self.current {
                let windows = workspace.len();

                for (index, window) in workspace.iter_mut().enumerate() {
                    let win = (index + 1 < windows).then(|| area.split()).unwrap_or(area);

                    window.mov_resize(win.x, win.y, win.width, win.height)?;

                    window.map(WindowKind::Window)?;
                }
            } else {
                for window in workspace {
                    window.unmap(WindowKind::Window)?;
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
            width: self.width - padding.right,
            height: self.height - padding.bottom,
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
        self.root.select_input(&[EventMask::SubstructureNotify, EventMask::SubstructureRedirect, EventMask::EnterWindow])?;

        self.server.listen()?;

        startup::startup()?;

        Ok(())
    }

    fn tile(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.monitors.all(|monitor| {
            monitor.workspace.tile(monitor.area.pad(self.config.padding))
        })?;

        Ok(())
    }

    fn focused_win<F>(&mut self, f: F) -> Result<(), Box<dyn std::error::Error>> where F: Fn(Window<UnixStream>) -> Result<(), Box<dyn std::error::Error>> {
        let focus = self.display.get_input_focus()?;

        if focus.window != self.root.id() {
            f(self.display.window_from_id(focus.window)?)
        } else {
            Ok(())
        }
    }

    fn handle_incoming(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for sequence in self.server.incoming()? {
            println!("sequence: {:?}", sequence);

            match sequence.request {
                Request::Workspace => {
                    self.monitors.focused(|monitor| {
                        monitor.workspace.current = sequence.value as usize;

                        monitor.workspace.tile(monitor.area)
                    })?;
                },
                Request::Kill => {
                    self.focused_win(|mut window| window.kill())?;
                },
                Request::Close => {
                },
                Request::PaddingTop => self.config.padding.top = sequence.value as u16,
                Request::PaddingBottom => self.config.padding.bottom = sequence.value as u16,
                Request::PaddingLeft => self.config.padding.left = sequence.value as u16,
                Request::PaddingRight => self.config.padding.right = sequence.value as u16,
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
                    monitor.workspace.insert(self.display.window_from_id(window)?);

                    Ok(())
                })?;

                self.tile()?;

                let mut window = self.display.window_from_id(window)?;

                window.select_input(&[EventMask::SubstructureNotify, EventMask::SubstructureRedirect, EventMask::EnterWindow])?;

                window.set_input_focus(RevertTo::Parent)?;
            },
            Event::UnmapNotify { window, .. } => {
                log::write(format!("unmap notify: {}\n", window), Severity::Info)?;

                self.monitors.all(|monitor| {
                    if let Some(index) = monitor.workspace.find(window) {
                        monitor.workspace.remove(index);

                        monitor.workspace.set_nearest_input_focus(index)?;
                    }

                    Ok(())
                })?;

                self.tile()?;
            },
            Event::EnterNotify { window, .. } => {
                if self.root.id() != window {
                    self.display.window_from_id(window)?.set_input_focus(RevertTo::Parent)?;
                }
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


