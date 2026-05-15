#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HueColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl HueColor {
    pub const RED: Self = Self {
        red: 255,
        green: 0,
        blue: 0,
    };

    pub const GREEN: Self = Self {
        red: 0,
        green: 255,
        blue: 0,
    };

    pub const BLUE: Self = Self {
        red: 0,
        green: 0,
        blue: 255,
    };

    pub const WHITE: Self = Self {
        red: 255,
        green: 255,
        blue: 255,
    };

    pub fn parse_named(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "red" => Some(Self::RED),
            "green" => Some(Self::GREEN),
            "blue" => Some(Self::BLUE),
            "white" => Some(Self::WHITE),
            _ => None,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::parse_named(value).or_else(|| Self::parse_hex(value))
    }

    fn parse_hex(value: &str) -> Option<Self> {
        let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
        if value.len() != 6 || !value.is_ascii() {
            return None;
        }

        let red = u8::from_str_radix(&value[0..2], 16).ok()?;
        let green = u8::from_str_radix(&value[2..4], 16).ok()?;
        let blue = u8::from_str_radix(&value[4..6], 16).ok()?;

        Some(Self { red, green, blue })
    }
}

#[cfg(test)]
mod tests {
    use super::HueColor;

    #[test]
    fn parses_named_colors_case_insensitively() {
        assert_eq!(HueColor::parse_named("RED"), Some(HueColor::RED));
        assert_eq!(HueColor::parse_named("green"), Some(HueColor::GREEN));
        assert_eq!(HueColor::parse_named("unknown"), None);
    }

    #[test]
    fn parses_hex_colors() {
        assert_eq!(
            HueColor::parse("#ff8000"),
            Some(HueColor {
                red: 255,
                green: 128,
                blue: 0,
            })
        );
        assert_eq!(
            HueColor::parse("00ff7f"),
            Some(HueColor {
                red: 0,
                green: 255,
                blue: 127,
            })
        );
        assert_eq!(HueColor::parse("#bad"), None);
        assert_eq!(HueColor::parse("zzzzzz"), None);
    }
}
