

#[derive(Debug, Default, Clone, Copy)]
pub struct Padding {
    pub top: u16,
    pub bottom: u16,
    pub left: u16,
    pub right: u16,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Borders {
    pub width: u16,
    pub focused: u32,
    pub normal: u32,
}

#[derive(Debug, Default)]
pub struct Windows {
    pub borders: Borders,
    pub gaps: u16,
    pub mouse_movement: bool,
}

#[derive(Debug, Default)]
pub struct Config {
    pub padding: Padding,
    pub windows: Windows,
}


