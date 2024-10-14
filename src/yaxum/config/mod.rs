

#[derive(Debug, Default, Clone, Copy)]
pub struct Padding {
    pub top: u16,
    pub bottom: u16,
    pub left: u16,
    pub right: u16,
}

#[derive(Debug, Default)]
pub struct Borders {
    pub width: usize,
}

#[derive(Debug, Default)]
pub struct Windows {
    pub borders: Borders,
    pub gaps: u8,
}

#[derive(Debug, Default)]
pub struct Config {
    pub padding: Padding,
    pub windows: Windows,
}


