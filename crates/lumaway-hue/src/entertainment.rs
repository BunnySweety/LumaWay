use crate::{HueColor, HueError, Result};

const HEADER: &[u8; 9] = b"HueStream";
const VERSION_MAJOR: u8 = 0x02;
const VERSION_MINOR: u8 = 0x00;
const MESSAGE_HEADER_SIZE: usize = 52;
const CHANNEL_DATA_SIZE: usize = 7;
const CONFIG_ID_SIZE: usize = 36;
const COLORSPACE_RGB: u8 = 0x00;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelColor {
    pub channel_id: u8,
    pub color: HueColor,
}

#[derive(Debug, Clone)]
pub struct HueStreamFrame {
    pub entertainment_config_id: String,
    pub sequence: u8,
    pub channels: Vec<ChannelColor>,
}

#[derive(Debug, Clone)]
pub struct HueStreamEncoder {
    config_id: [u8; CONFIG_ID_SIZE],
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueStreamMessage {
    bytes: Vec<u8>,
}

impl HueStreamMessage {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl HueStreamFrame {
    pub fn encode_rgb(&self) -> Result<HueStreamMessage> {
        let config_id = self.entertainment_config_id.as_bytes();
        validate_config_id(config_id, &self.entertainment_config_id)?;

        let mut bytes = vec![0u8; MESSAGE_HEADER_SIZE + CHANNEL_DATA_SIZE * self.channels.len()];
        write_rgb_frame(&mut bytes, config_id, self.sequence, &self.channels);

        Ok(HueStreamMessage { bytes })
    }
}

impl HueStreamEncoder {
    pub fn new(entertainment_config_id: &str, channel_count: usize) -> Result<Self> {
        let config_id = entertainment_config_id.as_bytes();
        validate_config_id(config_id, entertainment_config_id)?;

        let mut stored_config_id = [0u8; CONFIG_ID_SIZE];
        stored_config_id.copy_from_slice(config_id);
        let bytes = vec![0u8; MESSAGE_HEADER_SIZE + CHANNEL_DATA_SIZE * channel_count];
        Ok(Self {
            config_id: stored_config_id,
            bytes,
        })
    }

    pub fn encode_rgb(&mut self, sequence: u8, channels: &[ChannelColor]) -> &[u8] {
        let required_len = MESSAGE_HEADER_SIZE + CHANNEL_DATA_SIZE * channels.len();
        if self.bytes.len() != required_len {
            self.bytes.resize(required_len, 0);
        }

        write_rgb_frame(&mut self.bytes, &self.config_id, sequence, channels);
        &self.bytes
    }
}

fn validate_config_id(config_id: &[u8], original: &str) -> Result<()> {
    if config_id.len() != CONFIG_ID_SIZE || !config_id.is_ascii() {
        return Err(HueError::InvalidEntertainmentConfigId(original.to_string()));
    }

    Ok(())
}

fn write_rgb_frame(bytes: &mut [u8], config_id: &[u8], sequence: u8, channels: &[ChannelColor]) {
    bytes.fill(0);
    bytes[0..HEADER.len()].copy_from_slice(HEADER);
    bytes[9] = VERSION_MAJOR;
    bytes[10] = VERSION_MINOR;
    bytes[11] = sequence;
    bytes[14] = COLORSPACE_RGB;
    bytes[16..52].copy_from_slice(config_id);

    let mut offset = MESSAGE_HEADER_SIZE;
    for channel in channels {
        bytes[offset] = channel.channel_id;
        write_u16_be(bytes, offset + 1, scale_color(channel.color.red));
        write_u16_be(bytes, offset + 3, scale_color(channel.color.green));
        write_u16_be(bytes, offset + 5, scale_color(channel.color.blue));
        offset += CHANNEL_DATA_SIZE;
    }
}

fn scale_color(value: u8) -> u16 {
    // Match Lumux: 8-bit channel -> 16-bit via `v * 257` (equivalent to `round(v * 65535 / 255)`).
    u16::from(value).saturating_mul(257)
}

fn write_u16_be(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset] = (value >> 8) as u8;
    bytes[offset + 1] = value as u8;
}

#[cfg(test)]
mod tests {
    use super::{
        scale_color, ChannelColor, HueStreamEncoder, HueStreamFrame, CHANNEL_DATA_SIZE, HEADER,
        MESSAGE_HEADER_SIZE,
    };
    use crate::{HueColor, HueError};

    const CONFIG_ID: &str = "01234567-89ab-cdef-0123-456789abcdef";

    #[test]
    fn scales_8_bit_colors_to_16_bit_values() {
        assert_eq!(scale_color(0), 0);
        assert_eq!(scale_color(1), 257);
        assert_eq!(scale_color(128), 32_896);
        assert_eq!(scale_color(255), 65_535);
    }

    #[test]
    fn encodes_rgb_frame_header() {
        let message = HueStreamFrame {
            entertainment_config_id: CONFIG_ID.to_string(),
            sequence: 7,
            channels: Vec::new(),
        }
        .encode_rgb()
        .unwrap();

        let bytes = message.as_bytes();
        assert_eq!(&bytes[0..9], HEADER);
        assert_eq!(bytes[9], 0x02);
        assert_eq!(bytes[10], 0x00);
        assert_eq!(bytes[11], 7);
        assert_eq!(bytes[14], 0x00);
        assert_eq!(&bytes[16..52], CONFIG_ID.as_bytes());
        assert_eq!(bytes.len(), MESSAGE_HEADER_SIZE);
    }

    #[test]
    fn encodes_rgb_channel_payload() {
        let message = HueStreamFrame {
            entertainment_config_id: CONFIG_ID.to_string(),
            sequence: 1,
            channels: vec![
                ChannelColor {
                    channel_id: 3,
                    color: HueColor::RED,
                },
                ChannelColor {
                    channel_id: 4,
                    color: HueColor {
                        red: 0,
                        green: 128,
                        blue: 255,
                    },
                },
            ],
        }
        .encode_rgb()
        .unwrap();

        let bytes = message.as_bytes();
        assert_eq!(bytes.len(), MESSAGE_HEADER_SIZE + CHANNEL_DATA_SIZE * 2);

        let first = MESSAGE_HEADER_SIZE;
        assert_eq!(bytes[first], 3);
        assert_eq!(&bytes[first + 1..first + 3], &[0xff, 0xff]);
        assert_eq!(&bytes[first + 3..first + 5], &[0x00, 0x00]);
        assert_eq!(&bytes[first + 5..first + 7], &[0x00, 0x00]);

        let second = MESSAGE_HEADER_SIZE + CHANNEL_DATA_SIZE;
        assert_eq!(bytes[second], 4);
        assert_eq!(&bytes[second + 1..second + 3], &[0x00, 0x00]);
        assert_eq!(&bytes[second + 3..second + 5], &[0x80, 0x80]);
        assert_eq!(&bytes[second + 5..second + 7], &[0xff, 0xff]);
    }

    #[test]
    fn rejects_invalid_config_id_length() {
        let err = HueStreamFrame {
            entertainment_config_id: "not-a-uuid".to_string(),
            sequence: 1,
            channels: Vec::new(),
        }
        .encode_rgb()
        .unwrap_err();

        assert!(matches!(err, HueError::InvalidEntertainmentConfigId(_)));
    }

    #[test]
    fn reusable_encoder_updates_sequence_and_channels() {
        let mut encoder = HueStreamEncoder::new(CONFIG_ID, 2).unwrap();
        let first = encoder
            .encode_rgb(
                1,
                &[
                    ChannelColor {
                        channel_id: 1,
                        color: HueColor::RED,
                    },
                    ChannelColor {
                        channel_id: 2,
                        color: HueColor {
                            red: 0,
                            green: 0,
                            blue: 255,
                        },
                    },
                ],
            )
            .to_vec();
        let second = encoder
            .encode_rgb(
                2,
                &[ChannelColor {
                    channel_id: 3,
                    color: HueColor {
                        red: 0,
                        green: 255,
                        blue: 0,
                    },
                }],
            )
            .to_vec();

        assert_eq!(first[11], 1);
        assert_eq!(first.len(), MESSAGE_HEADER_SIZE + CHANNEL_DATA_SIZE * 2);
        assert_eq!(second[11], 2);
        assert_eq!(second.len(), MESSAGE_HEADER_SIZE + CHANNEL_DATA_SIZE);
        assert_eq!(second[MESSAGE_HEADER_SIZE], 3);
        assert_eq!(
            &second[MESSAGE_HEADER_SIZE + 3..MESSAGE_HEADER_SIZE + 5],
            &[0xff, 0xff]
        );
    }
}
