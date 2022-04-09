#[derive(Clone, Copy, Debug)]
struct Rgba {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

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

impl Header {
    fn parse<Source: Iterator<Item = u8>>(source: &mut Source) -> Option<Header> {
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
        let mut result = [Default::default(); 14];
        result[0..4] = {}
        result
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

    pub fn hash_index(&self) -> u8 {
        // Wrapping is fine, as range(u8)=256 is a multiple of 64 anyway
        (self.red.wrapping_mul(3)
            + self.green.wrapping_mul(5)
            + self.blue.wrapping_mul(7)
            + self.alpha.wrapping_mul(11))
            % 64
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
}

struct EnDecoder {
    last_seen_pixel: Rgba,
    stored_pixels: [Rgba; 64],
}

enum DecodeError {
    MissingTerminator,
    Header,
    IllegalRun,
    ConsecutiveIndex,
}

impl EnDecoder {
    pub fn new() -> Self {
        EnDecoder {
            last_seen_pixel: Default::default(),
            // TODO: Correct, or should just be completely zero? Standard says zero-initialized..
            stored_pixels: [Default::default(); 64],
        }
    }

    pub fn reset(&mut self) {
        std::mem::swap(self, &mut Default::default());
    }

    pub fn decode_into<So: Iterator<Item = u8>, Si: Sink<Rgba>>(
        &mut self,
        source: &mut So,
        sink: &mut Si,
    ) -> Result<Header, DecodeError> {
        if let Some(header) = Header::parse(source) {
            Ok(header)
        } else {
            Err(DecodeError::Header)
        }
    }
}

impl Default for EnDecoder {
    fn default() -> Self {
        EnDecoder::new()
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
