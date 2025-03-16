//https://developer.gnome.org/hig/reference/palette.html#palette

#[allow(dead_code)]
#[derive(Debug)]
pub enum Palette<'a> {
    Custom(&'a str),
    Blue1,
    Blue2,
    Blue3,
    Blue4,
    Blue5,
    Green1,
    Green2,
    Green3,
    Green4,
    Green5,
    Yellow1,
    Yellow2,
    Yellow3,
    Yellow4,
    Yellow5,
    Orange1,
    Orange2,
    Orange3,
    Orange4,
    Orange5,
    Red1,
    Red2,
    Red3,
    Red4,
    Red5,
    RedErrorDark,
    Purple1,
    Purple2,
    Purple3,
    Purple4,
    Purple5,
    Brown1,
    Brown2,
    Brown3,
    Brown4,
    Brown5,
    Light1,
    Light2,
    Light3,
    Light4,
    Light5,
    Dark1,
    Dark2,
    Dark3,
    Dark4,
    Dark5,
}

impl<'a> Palette<'a> {
    pub fn get_color(&self) -> &'a str {
        match *self {
            Palette::Custom(cus) => cus,
            Palette::Blue1 => "#99c1f1",
            Palette::Blue2 => "#62a0ea",
            Palette::Blue3 => "#3584e4",
            Palette::Blue4 => "#1c71d8",
            Palette::Blue5 => "#1a5fb4",
            Palette::Green1 => "#8ff0a4",
            Palette::Green2 => "#57e389",
            Palette::Green3 => "#33d17a",
            Palette::Green4 => "#2ec27e",
            Palette::Green5 => "#26a269",
            Palette::Yellow1 => "#f9f06b",
            Palette::Yellow2 => "#f8e45c",
            Palette::Yellow3 => "#f6d32d",
            Palette::Yellow4 => "#f5c211",
            Palette::Yellow5 => "#e5a50a",
            Palette::Orange1 => "#ffbe6f",
            Palette::Orange2 => "#ffa348",
            Palette::Orange3 => "#ff7800",
            Palette::Orange4 => "#e66100",
            Palette::Orange5 => "#c64600",
            Palette::Red1 => "#f66151",
            Palette::Red2 => "#ed333b",
            Palette::Red3 => "#e01b24",
            Palette::Red4 => "#c01c28",
            Palette::Red5 => "#a51d2d",
            Palette::RedErrorDark => "#ff888c",
            Palette::Purple1 => "#dc8add",
            Palette::Purple2 => "#c061cb",
            Palette::Purple3 => "#9141ac",
            Palette::Purple4 => "#813d9c",
            Palette::Purple5 => "#613583",
            Palette::Brown1 => "#cdab8f",
            Palette::Brown2 => "#b5835a",
            Palette::Brown3 => "#986a44",
            Palette::Brown4 => "#865e3c",
            Palette::Brown5 => "#63452c",
            Palette::Light1 => "#ffffff",
            Palette::Light2 => "#f6f5f4",
            Palette::Light3 => "#deddda",
            Palette::Light4 => "#c0bfbc",
            Palette::Light5 => "#9a9996",
            Palette::Dark1 => "#77767b",
            Palette::Dark2 => "#5e5c64",
            Palette::Dark3 => "#3d3846",
            Palette::Dark4 => "#241f31",
            Palette::Dark5 => "#000000",
        }
    }

    pub fn get_rgb(&self) -> (u8, u8, u8) {
        let color = self.get_color();

        let r = u8::from_str_radix(&color[1..=2], 16).unwrap();
        let g = u8::from_str_radix(&color[3..=4], 16).unwrap();
        let b = u8::from_str_radix(&color[5..=6], 16).unwrap();

        (r, g, b)
    }

    pub fn get_rgb_u16(&self) -> (u16, u16, u16) {
        let (r, g, b) = self.get_rgb();

        let r16: u16 = ((r as u16) << 8) | r as u16;
        let g16: u16 = ((g as u16) << 8) | g as u16;
        let b16: u16 = ((b as u16) << 8) | b as u16;
        (r16, g16, b16)
    }
}

pub fn grey(is_dark: bool) -> Palette<'static> {
    if is_dark {
        Palette::Light5
    } else {
        Palette::Dark1
    }
}

pub fn red(is_dark: bool) -> Palette<'static> {
    if is_dark {
        Palette::Custom("#ff938c")
    } else {
        Palette::Custom("#c30000")
    }
}

pub fn yellow(is_dark: bool) -> Palette<'static> {
    if is_dark {
        Palette::Custom("#ffc252")
    } else {
        Palette::Custom("#905400")
    }
}

pub fn green(is_dark: bool) -> Palette<'static> {
    if is_dark {
        Palette::Custom("#78e9ab")
    } else {
        Palette::Custom("#007c3d")
    }
}
