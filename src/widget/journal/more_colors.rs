#![allow(dead_code)]
use super::colorise::TermColor;

pub fn get_256color(code: u8) -> TermColor {
    let color = match code {
        0 => TermColor::Black,
        1 => TermColor::Red,
        2 => TermColor::Green,
        3 => TermColor::Yellow,
        4 => TermColor::Blue,
        5 => TermColor::Magenta,
        6 => TermColor::Cyan,
        7 => TermColor::White,
        8 => TermColor::BrightBlack,
        9 => TermColor::BrightRed,
        10 => TermColor::BrightGreen,
        11 => TermColor::BrightYellow,
        12 => TermColor::BrightBlue,
        13 => TermColor::BrightMagenta,
        14 => TermColor::BrightCyan,
        15 => TermColor::BrightWhite,
        16..=231 => color_map216(code),
        232..=255 => grayscale(code),
    };

    color
}

fn grayscale(code: u8) -> TermColor {
    let gray_scale: u8 = (code - 232) * 10 + 8;

    TermColor::VGA(gray_scale, gray_scale, gray_scale)
}

fn color_map216(code: u8) -> TermColor {
    let base = code - 16;

    let mut r: u8 = (base / 36) * 40;
    if r > 0 {
        r += 55
    }

    let mut g: u8 = ((base % 36) / 6) * 40;
    if g > 0 {
        g += 55
    }

    let mut b: u8 = (base % 6) * 40;

    if b > 0 {
        b += 55
    }

    TermColor::VGA(r, g, b)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_color_map() {
        let serie = [
            (16, "#000000"),
            (52, "#5f0000"),
            (160, "#d70000"),
            (196, "#ff0000"),
            (22, "#005f00"),
            (46, "#00ff00"),
            (17, "#00005f"),
            (146, "#afafd7"),
        ];

        for elem in serie {
            let color = color_map216(elem.0);
            let hex_val = color.get_hexa_code();

            assert_eq!(hex_val, elem.1, "code {}", elem.0)
        }
    }

    #[test]
    fn test_grayscale() {
        let serie = [
            (232, "#080808"),
            (233, "#121212"),
            (254, "#e4e4e4"),
            (255, "#eeeeee"),
        ];

        for elem in serie {
            let color = grayscale(elem.0);
            let hex_val = color.get_hexa_code();

            assert_eq!(hex_val, elem.1, "code {}", elem.0)
        }
    }
}
