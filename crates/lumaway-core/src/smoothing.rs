use crate::RgbAverage;

#[derive(Debug, Clone)]
pub struct ColorSmoother {
    config: ColorSmoothingConfig,
    previous: Vec<RgbAverage>,
}

#[derive(Debug, Clone, Copy)]
pub struct ColorSmoothingConfig {
    pub alpha: f64,
    pub noise_threshold: u8,
    pub max_step: Option<u8>,
}

impl ColorSmoother {
    pub fn new(alpha: f64) -> Self {
        Self::with_noise_threshold(alpha, 0)
    }

    pub fn with_noise_threshold(alpha: f64, noise_threshold: u8) -> Self {
        Self::with_config(ColorSmoothingConfig {
            alpha,
            noise_threshold,
            max_step: None,
        })
    }

    pub fn with_config(config: ColorSmoothingConfig) -> Self {
        Self {
            config: ColorSmoothingConfig {
                alpha: config.alpha.clamp(0.0, 1.0),
                noise_threshold: config.noise_threshold,
                max_step: config.max_step,
            },
            previous: Vec::new(),
        }
    }

    pub fn smooth(&mut self, colors: Vec<RgbAverage>) -> Vec<RgbAverage> {
        if self.previous.len() != colors.len() {
            self.previous = colors.clone();
            return colors;
        }

        let smoothed: Vec<_> = colors
            .into_iter()
            .zip(self.previous.iter().copied())
            .map(|(current, previous)| {
                if color_delta(previous, current) <= self.config.noise_threshold {
                    previous
                } else {
                    let smoothed = smooth_color(previous, current, self.config.alpha);
                    limit_color_step(previous, smoothed, self.config.max_step)
                }
            })
            .collect();
        self.previous = smoothed.clone();
        smoothed
    }
}

fn limit_color_step(previous: RgbAverage, current: RgbAverage, max_step: Option<u8>) -> RgbAverage {
    let Some(max_step) = max_step else {
        return current;
    };

    RgbAverage {
        red: limit_channel_step(previous.red, current.red, max_step),
        green: limit_channel_step(previous.green, current.green, max_step),
        blue: limit_channel_step(previous.blue, current.blue, max_step),
    }
}

fn limit_channel_step(previous: u8, current: u8, max_step: u8) -> u8 {
    if current > previous {
        current.min(previous.saturating_add(max_step))
    } else {
        current.max(previous.saturating_sub(max_step))
    }
}

fn color_delta(previous: RgbAverage, current: RgbAverage) -> u8 {
    previous
        .red
        .abs_diff(current.red)
        .max(previous.green.abs_diff(current.green))
        .max(previous.blue.abs_diff(current.blue))
}

fn smooth_color(previous: RgbAverage, current: RgbAverage, alpha: f64) -> RgbAverage {
    RgbAverage {
        red: smooth_channel(previous.red, current.red, alpha),
        green: smooth_channel(previous.green, current.green, alpha),
        blue: smooth_channel(previous.blue, current.blue, alpha),
    }
}

fn smooth_channel(previous: u8, current: u8, alpha: f64) -> u8 {
    ((f64::from(previous) * (1.0 - alpha)) + (f64::from(current) * alpha)).round() as u8
}

#[cfg(test)]
mod tests {
    use super::{ColorSmoother, ColorSmoothingConfig};
    use crate::RgbAverage;

    #[test]
    fn first_frame_passes_through() {
        let mut smoother = ColorSmoother::new(0.35);
        let colors = vec![RgbAverage {
            red: 10,
            green: 20,
            blue: 30,
        }];

        assert_eq!(smoother.smooth(colors.clone()), colors);
    }

    #[test]
    fn applies_exponential_smoothing_per_channel() {
        let mut smoother = ColorSmoother::new(0.25);
        smoother.smooth(vec![RgbAverage {
            red: 0,
            green: 100,
            blue: 200,
        }]);

        let smoothed = smoother.smooth(vec![RgbAverage {
            red: 100,
            green: 0,
            blue: 0,
        }]);

        assert_eq!(
            smoothed,
            vec![RgbAverage {
                red: 25,
                green: 75,
                blue: 150,
            }]
        );
    }

    #[test]
    fn alpha_one_uses_current_frame() {
        let mut smoother = ColorSmoother::new(1.0);
        smoother.smooth(vec![RgbAverage {
            red: 0,
            green: 0,
            blue: 0,
        }]);

        assert_eq!(
            smoother.smooth(vec![RgbAverage {
                red: 255,
                green: 128,
                blue: 64,
            }]),
            vec![RgbAverage {
                red: 255,
                green: 128,
                blue: 64,
            }]
        );
    }

    #[test]
    fn channel_count_change_resets_state() {
        let mut smoother = ColorSmoother::new(0.1);
        smoother.smooth(vec![RgbAverage {
            red: 0,
            green: 0,
            blue: 0,
        }]);

        let colors = vec![
            RgbAverage {
                red: 255,
                green: 0,
                blue: 0,
            },
            RgbAverage {
                red: 0,
                green: 0,
                blue: 255,
            },
        ];

        assert_eq!(smoother.smooth(colors.clone()), colors);
    }

    #[test]
    fn ignores_changes_within_noise_threshold() {
        let mut smoother = ColorSmoother::with_noise_threshold(1.0, 4);
        smoother.smooth(vec![RgbAverage {
            red: 100,
            green: 120,
            blue: 140,
        }]);

        assert_eq!(
            smoother.smooth(vec![RgbAverage {
                red: 103,
                green: 116,
                blue: 141,
            }]),
            vec![RgbAverage {
                red: 100,
                green: 120,
                blue: 140,
            }]
        );
    }

    #[test]
    fn accepts_changes_above_noise_threshold() {
        let mut smoother = ColorSmoother::with_noise_threshold(1.0, 4);
        smoother.smooth(vec![RgbAverage {
            red: 100,
            green: 120,
            blue: 140,
        }]);

        assert_eq!(
            smoother.smooth(vec![RgbAverage {
                red: 105,
                green: 120,
                blue: 140,
            }]),
            vec![RgbAverage {
                red: 105,
                green: 120,
                blue: 140,
            }]
        );
    }

    #[test]
    fn limits_per_frame_step_up_and_down() {
        let mut smoother = ColorSmoother::with_config(ColorSmoothingConfig {
            alpha: 1.0,
            noise_threshold: 0,
            max_step: Some(10),
        });
        smoother.smooth(vec![RgbAverage {
            red: 100,
            green: 100,
            blue: 100,
        }]);

        assert_eq!(
            smoother.smooth(vec![RgbAverage {
                red: 255,
                green: 0,
                blue: 105,
            }]),
            vec![RgbAverage {
                red: 110,
                green: 90,
                blue: 105,
            }]
        );
    }

    #[test]
    fn max_step_none_keeps_smoothed_value() {
        let mut smoother = ColorSmoother::with_config(ColorSmoothingConfig {
            alpha: 1.0,
            noise_threshold: 0,
            max_step: None,
        });
        smoother.smooth(vec![RgbAverage {
            red: 100,
            green: 100,
            blue: 100,
        }]);

        assert_eq!(
            smoother.smooth(vec![RgbAverage {
                red: 255,
                green: 0,
                blue: 105,
            }]),
            vec![RgbAverage {
                red: 255,
                green: 0,
                blue: 105,
            }]
        );
    }
}
