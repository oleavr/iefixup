use alloc::vec::Vec;

const REPLACEMENT_CHARACTER: u8 = b'?';

const ASCII_PUNCTUATION: &[(u32, u8)] = &[
    (0x2010, b'-'),
    (0x2011, b'-'),
    (0x2012, b'-'),
    (0x2013, b'-'),
    (0x2014, b'-'),
    (0x2015, b'-'),
    (0x2018, b'\''),
    (0x2019, b'\''),
    (0x201a, b'\''),
    (0x201c, b'"'),
    (0x201d, b'"'),
    (0x201e, b'"'),
    (0x2022, b'*'),
    (0x2026, b'.'),
    (0x2039, b'<'),
    (0x203a, b'>'),
    (0x2190, b'<'),
    (0x2192, b'>'),
    (0x2009, b' '),
    (0x200a, b' '),
    (0x202f, b' '),
    (0x2212, b'-'),
];

const CP1252_UPPER_HALF: [u16; 32] = [
    0x20ac, 0x0000, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x0000, 0x017d, 0x0000, 0x0000, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014,
    0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0x0000, 0x017e, 0x0178,
];

#[derive(Default)]
pub struct Utf8Decoder {
    codepoint: u32,
    pending: u32,
}

impl Utf8Decoder {
    pub fn feed(&mut self, byte: u8, output: &mut Vec<u8>) {
        if byte & 0xc0 == 0x80 {
            if self.pending == 0 {
                output.push(REPLACEMENT_CHARACTER);
                return;
            }
            self.codepoint = (self.codepoint << 6) | u32::from(byte & 0x3f);
            self.pending -= 1;
            if self.pending == 0 {
                output.push(cp1252_from_codepoint(self.codepoint));
            }
            return;
        }

        self.flush(output);

        let (bits, pending) = match byte {
            0x00..=0x7f => {
                output.push(byte);
                return;
            }
            _ if byte & 0xe0 == 0xc0 => (byte & 0x1f, 1),
            _ if byte & 0xf0 == 0xe0 => (byte & 0x0f, 2),
            _ if byte & 0xf8 == 0xf0 => (byte & 0x07, 3),
            _ => {
                output.push(REPLACEMENT_CHARACTER);
                return;
            }
        };
        self.codepoint = u32::from(bits);
        self.pending = pending;
    }

    pub fn flush(&mut self, output: &mut Vec<u8>) {
        if self.pending != 0 {
            output.push(REPLACEMENT_CHARACTER);
        }
        self.pending = 0;
    }
}

pub fn append_as_cp1252(output: &mut Vec<u8>, data: &[u8]) {
    let mut decoder = Utf8Decoder::default();
    for &byte in data {
        decoder.feed(byte, output);
    }
    decoder.flush(output);
}

fn cp1252_from_codepoint(codepoint: u32) -> u8 {
    if codepoint < 0x80 || (0xa0..=0xff).contains(&codepoint) {
        return codepoint as u8;
    }

    if let Some(&(_, ascii)) = ASCII_PUNCTUATION.iter().find(|&&(c, _)| c == codepoint) {
        return ascii;
    }

    match CP1252_UPPER_HALF
        .iter()
        .position(|&c| u32::from(c) == codepoint)
    {
        Some(index) => 0x80 + index as u8,
        None => REPLACEMENT_CHARACTER,
    }
}
