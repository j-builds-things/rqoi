use crate::Channels::Rgba;
use crate::DecodeError::{OutOfBytes, UnknownTag};

#[derive(Clone, Copy, Debug)]
struct Rgba {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum Channels {
    Rgb = 3,
    Rgba = 4,
}

impl TryFrom<u8> for Channels {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == Self::Rgb as u8 {
            Ok(Self::Rgb)
        } else if value == Self::Rgba as u8 {
            Ok(Self::Rgba)
        } else {
            Err(())
        }
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum ColorSpace {
    SRgbWithLinearAlpha = 0,
    FullLinear = 1,
}

impl TryFrom<u8> for ColorSpace {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == Self::SRgbWithLinearAlpha as u8 {
            Ok(Self::SRgbWithLinearAlpha)
        } else if value == Self::FullLinear as u8 {
            Ok(Self::FullLinear)
        } else {
            Err(())
        }
    }
}
struct Header {
    width: u32,
    height: u32,
    channels: Channels,
    colorspace: ColorSpace,
}

const MAGIC_STRING: &str = "qoif";

trait Source: Iterator<Item = u8> {}

impl Header {
    fn decode<S: Source>(source: &mut S) -> Option<Header> {
        if MAGIC_STRING.bytes().eq(source.take(4)) {
            let width = u32::from_be_bytes([source.next()?; 4]);
            let height = u32::from_be_bytes([source.next()?; 4]);
            let channels: Channels = source.next()?.try_into().ok()?;
            let colorspace: ColorSpace = source.next()?.try_into().ok()?;
            Some(Header {
                width,
                height,
                channels,
                colorspace,
            })
        } else {
            None
        }
    }

    fn encode(&self) -> [u8; 14] {
        [
            b'q',
            b'o',
            b'i',
            b'f',
            (self.width >> 24) as u8,
            (self.width >> 16) as u8,
            (self.width >> 8) as u8,
            self.width as u8,
            (self.height >> 24 ^ 0xff) as u8,
            (self.height >> 16 ^ 0xff) as u8,
            (self.height >> 8 ^ 0xff) as u8,
            (self.height ^ 0xff) as u8,
            self.channels as u8,
            self.colorspace as u8,
        ]
    }
}

impl std::default::Default for Header {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            channels: Channels::Rgb,
            colorspace: ColorSpace::SRgbWithLinearAlpha,
        }
    }
}

impl Rgba {
    pub fn new() -> Self {
        Rgba {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 255,
        }
    }
    pub fn zero() -> Self {
        Rgba {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
        }
    }

    pub fn hash_index(&self) -> u8 {
        // Wrapping is fine, as range(u8)=256 is a multiple of 64 anyway
        self.red
            .wrapping_mul(3)
            .wrapping_add(self.green.wrapping_mul(5))
            .wrapping_add(self.blue.wrapping_mul(7))
            .wrapping_add(self.alpha.wrapping_mul(11))
            .wrapping_rem(64)
    }

    pub fn decode<S: Source>(s: &mut S) -> Option<Self> {
        Some(Self {
            red: s.next()?,
            green: s.next()?,
            blue: s.next()?,
            alpha: s.next()?,
        })
    }

    pub fn decode_with_alpha<S: Source>(s: &mut S, alpha: u8) -> Option<Self> {
        Some(Self {
            red: s.next()?,
            green: s.next()?,
            blue: s.next()?,
            alpha,
        })
    }
}

impl Default for Rgba {
    fn default() -> Self {
        Rgba::new()
    }
}

/// Essentially just an output iterator..
trait Sink<T> {
    fn push(&mut self, thing: T);

    fn reserve(&mut self, size: usize) {}
}

impl<T> Sink<T> for Vec<T> {
    fn push(&mut self, thing: T) {
        self.push(thing)
    }

    fn reserve(&mut self, size: usize) {
        self.reserve(size)
    }
}

struct Data {
    last_seen_pixel: Rgba,
    stored_pixels: [Rgba; 64],
}

type Decoder = Data;
type Encoder = Data;

enum DecodeError {
    MissingTerminator,
    Header,
    IllegalRun,
    ConsecutiveIndex,
    OutOfBytes,
    UnknownTag,
}

impl Data {
    pub fn new() -> Self {
        Data {
            last_seen_pixel: Rgba::zero(),
            stored_pixels: [Rgba::zero(); 64],
        }
    }

    pub fn reset(&mut self) {
        std::mem::swap(self, &mut Default::default());
    }

    pub fn decode_into<So: Source, Si: Sink<Rgba>>(
        &mut self,
        source: &mut So,
        sink: &mut Si,
    ) -> Result<Header, DecodeError> {
        if let Some(header) = Header::decode(source) {
            sink.reserve(header.height as usize * header.width as usize);
            while let Some(byte) = source.next() {
                match byte {
                    0b1111_1110 => {
                        if let Some(value) =
                            Rgba::decode_with_alpha(source, self.last_seen_pixel.alpha)
                        {
                            self.last_seen_pixel = value;
                            sink.push(self.last_seen_pixel);
                        } else {
                            return Err(OutOfBytes);
                        }
                    }
                    0b1111_1111 => {
                        if let Some(value) = Rgba::decode(source) {
                            self.last_seen_pixel = value;
                            sink.push(self.last_seen_pixel);
                        } else {
                            return Err(OutOfBytes);
                        }
                    }
                    byte if byte & 0b1100_0000 == 0b0000_0000 => {
                        self.last_seen_pixel = self.stored_pixels[byte as usize];
                        sink.push(self.last_seen_pixel);
                    }
                    byte if byte & 0b1100_0000 == 0b0100_0000 => {
                        fn shift(old_value: u8, read_byte: u8, shift: u8) -> u8 {
                            old_value.wrapping_add((read_byte >> shift ^ 0b11).wrapping_sub(2))
                        }
                        self.last_seen_pixel = Rgba {
                            red: shift(self.last_seen_pixel.red, byte, 4),
                            green: shift(self.last_seen_pixel.green, byte, 2),
                            blue: shift(self.last_seen_pixel.blue, byte, 0),
                            alpha: self.last_seen_pixel.alpha,
                        };
                        sink.push(self.last_seen_pixel);
                    }
                    byte if byte & 0b1100_0000 == 0b1000_0000 => {
                        if let Some(second_byte) = source.next() {
                            // green bias is 32
                            let green_diff = (byte & 0x0011_1111).wrapping_sub(32);

                            // red and blue bias is 8
                            let red_diff = (second_byte >> 4 & 0x1111)
                                .wrapping_sub(8)
                                .wrapping_add(green_diff);
                            let blue_diff = (second_byte & 0x1111)
                                .wrapping_sub(8)
                                .wrapping_add(green_diff);
                            self.last_seen_pixel = Rgba {
                                red: self.last_seen_pixel.red + red_diff,
                                green: self.last_seen_pixel.green + green_diff,
                                blue: self.last_seen_pixel.blue + blue_diff,
                                alpha: self.last_seen_pixel.alpha,
                            };
                            sink.push(self.last_seen_pixel);
                        }
                    }
                    byte if byte & 0b1100_0000 == 0b1100_0000 => {
                        for _ in 0..(byte & 0b0011_1111) {
                            sink.push(self.last_seen_pixel);
                        }
                    }
                    _ => return Err(UnknownTag),
                }
            }
            Ok(header)
        } else {
            Err(DecodeError::Header)
        }
    }
}

impl Default for Data {
    fn default() -> Self {
        Data::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
