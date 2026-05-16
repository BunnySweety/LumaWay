//! Capture RGB → graded Hue color (`ColorProfile`, `ColorTuning`).

use clap::ValueEnum;
use lumaway_core::RgbAverage;
use lumaway_hue::HueColor;

const DEFAULT_COLOR_GAIN: f64 = 1.8;
const DEFAULT_COLOR_GAMMA: f64 = 0.62;
const DEFAULT_COLOR_SATURATION: f64 = 1.45;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ColorProfile {
    Soft,
    Vivid,
    Game,
    Boosted,
    Cinema,
    Desktop,
}

#[derive(Debug, Clone, Copy)]
pub struct ColorTuning {
    pub(crate) gain: f64,
    pub(crate) gamma: f64,
    pub(crate) saturation: f64,
    pub(crate) black_threshold: u8,
    pub(crate) min_luma: f64,
}

impl From<ColorProfile> for ColorTuning {
    fn from(profile: ColorProfile) -> Self {
        match profile {
            ColorProfile::Soft => Self {
                gain: 1.25,
                gamma: 0.82,
                saturation: 1.15,
                black_threshold: 0,
                min_luma: 0.0,
            },
            ColorProfile::Vivid => Self {
                gain: DEFAULT_COLOR_GAIN,
                gamma: DEFAULT_COLOR_GAMMA,
                saturation: DEFAULT_COLOR_SATURATION,
                black_threshold: 3,
                min_luma: 36.0,
            },
            ColorProfile::Game => Self {
                gain: 1.9,
                gamma: 0.58,
                saturation: 1.55,
                black_threshold: 0,
                min_luma: 0.0,
            },
            ColorProfile::Boosted => Self {
                gain: 1.95,
                gamma: 0.56,
                saturation: 2.4,
                black_threshold: 0,
                min_luma: 0.0,
            },
            ColorProfile::Cinema => Self {
                gain: 1.55,
                gamma: 0.72,
                saturation: 1.25,
                black_threshold: 5,
                min_luma: 0.0,
            },
            ColorProfile::Desktop => Self {
                gain: 1.4,
                gamma: 0.78,
                saturation: 1.2,
                black_threshold: 0,
                min_luma: 0.0,
            },
        }
    }
}

pub fn hue_color_from_average(
    average: RgbAverage,
    brightness: f64,
    tuning: ColorTuning,
) -> HueColor {
    hue_color_from_graded(graded_color_from_average(average, tuning), brightness)
}

pub fn graded_color_from_average(average: RgbAverage, tuning: ColorTuning) -> HueColor {
    if max_rgb(average) <= tuning.black_threshold {
        return HueColor {
            red: 0,
            green: 0,
            blue: 0,
        };
    }

    let red = tune_color_channel(average.red, tuning);
    let green = tune_color_channel(average.green, tuning);
    let blue = tune_color_channel(average.blue, tuning);
    let luma = 0.2126 * red + 0.7152 * green + 0.0722 * blue;

    lift_luma_floor(
        HueColor {
            red: saturate_channel(red, luma, tuning.saturation),
            green: saturate_channel(green, luma, tuning.saturation),
            blue: saturate_channel(blue, luma, tuning.saturation),
        },
        tuning.min_luma,
    )
}

pub fn hue_color_from_graded(graded: HueColor, brightness: f64) -> HueColor {
    HueColor {
        red: apply_brightness(graded.red, brightness),
        green: apply_brightness(graded.green, brightness),
        blue: apply_brightness(graded.blue, brightness),
    }
}

fn max_rgb(average: RgbAverage) -> u8 {
    average.red.max(average.green).max(average.blue)
}

fn tune_color_channel(value: u8, tuning: ColorTuning) -> f64 {
    let normalized = (f64::from(value) / 255.0 * tuning.gain).clamp(0.0, 1.0);
    normalized.powf(tuning.gamma).clamp(0.0, 1.0) * 255.0
}

fn saturate_channel(value: f64, luma: f64, saturation: f64) -> u8 {
    (luma + (value - luma) * saturation)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn lift_luma_floor(color: HueColor, min_luma: f64) -> HueColor {
    if min_luma <= 0.0 {
        return color;
    }

    let luma = hue_luma(color);
    if luma <= 0.0 || luma >= min_luma {
        return color;
    }

    let scale = min_luma / luma;
    HueColor {
        red: scale_channel(color.red, scale),
        green: scale_channel(color.green, scale),
        blue: scale_channel(color.blue, scale),
    }
}

fn scale_channel(value: u8, scale: f64) -> u8 {
    (f64::from(value) * scale).round().clamp(0.0, 255.0) as u8
}

fn apply_brightness(value: u8, brightness: f64) -> u8 {
    (f64::from(value) * brightness).round().clamp(0.0, 255.0) as u8
}

pub fn rgb_luma(color: RgbAverage) -> f64 {
    0.2126 * f64::from(color.red) + 0.7152 * f64::from(color.green) + 0.0722 * f64::from(color.blue)
}

pub fn hue_luma(color: HueColor) -> f64 {
    0.2126 * f64::from(color.red) + 0.7152 * f64::from(color.green) + 0.0722 * f64::from(color.blue)
}

pub fn hue_saturation(color: HueColor) -> f64 {
    let max = f64::from(color.red.max(color.green).max(color.blue));
    if max == 0.0 {
        return 0.0;
    }
    let min = f64::from(color.red.min(color.green).min(color.blue));
    (max - min) / max
}
