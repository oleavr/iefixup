use crate::utf8::{self, Utf8Decoder};

use alloc::vec::Vec;

const MAX_TAG_LENGTH: usize = 65536;

const SUPPRESSED_ELEMENTS: &[&[u8]] = &[
    b"script",
    b"style",
    b"template",
    b"svg",
    b"iframe",
    b"object",
    b"canvas",
    b"video",
    b"audio",
];

const DROPPED_ELEMENTS: &[&[u8]] = &[
    b"link",
    b"noscript",
    b"picture",
    b"source",
    b"track",
    b"wbr",
];

const BLOCK_ELEMENTS: &[&[u8]] = &[
    b"header",
    b"nav",
    b"main",
    b"section",
    b"footer",
    b"article",
    b"aside",
    b"figure",
    b"figcaption",
    b"hgroup",
    b"details",
    b"summary",
    b"dialog",
    b"search",
];

const ALLOWED_ATTRIBUTES: &[&[u8]] = &[
    b"href",
    b"src",
    b"alt",
    b"name",
    b"title",
    b"width",
    b"height",
    b"border",
    b"align",
    b"valign",
    b"colspan",
    b"rowspan",
    b"nowrap",
    b"cellpadding",
    b"cellspacing",
    b"bgcolor",
    b"color",
    b"face",
    b"size",
    b"clear",
    b"hspace",
    b"vspace",
    b"usemap",
    b"shape",
    b"coords",
    b"target",
    b"action",
    b"method",
    b"type",
    b"value",
    b"maxlength",
    b"checked",
    b"selected",
    b"multiple",
    b"cols",
    b"rows",
    b"start",
    b"content",
    b"http-equiv",
];

#[derive(Default)]
pub struct HtmlRewriter {
    state: State,
    text_decoder: Utf8Decoder,
    tag: Vec<u8>,
    quote: u8,
    comment_dashes: u32,
    suppressed_element: Option<Vec<u8>>,
}

#[derive(Default, PartialEq)]
enum State {
    #[default]
    Text,
    Tag,
    Comment,
}

struct Tag<'a> {
    data: &'a [u8],
    closing: bool,
    self_closing: bool,
    name: &'a [u8],
    attributes_offset: usize,
}

struct Attribute<'a> {
    name: &'a [u8],
    raw: &'a [u8],
    value: &'a [u8],
}

struct Attributes<'a> {
    data: &'a [u8],
    offset: usize,
}

impl HtmlRewriter {
    pub fn feed(&mut self, data: &[u8], output: &mut Vec<u8>) {
        for &byte in data {
            self.process_byte(byte, output);
        }
    }

    pub fn finish(&mut self, output: &mut Vec<u8>) {
        if self.state == State::Tag {
            self.flush_tag(output);
        }
        self.text_decoder.flush(output);
    }

    fn process_byte(&mut self, byte: u8, output: &mut Vec<u8>) {
        match self.state {
            State::Text => self.process_text_byte(byte, output),
            State::Tag if self.suppressed_element.is_some() => {
                self.process_suppressed_tag_byte(byte, output)
            }
            State::Tag => self.process_tag_byte(byte, output),
            State::Comment => self.process_comment_byte(byte),
        }
    }

    fn process_text_byte(&mut self, byte: u8, output: &mut Vec<u8>) {
        if byte == b'<' {
            self.begin_tag(output);
        } else if self.suppressed_element.is_none() {
            self.text_decoder.feed(byte, output);
        }
    }

    fn begin_tag(&mut self, output: &mut Vec<u8>) {
        self.text_decoder.flush(output);

        self.state = State::Tag;
        self.tag.clear();
        self.tag.push(b'<');
        self.quote = 0;
    }

    fn process_tag_byte(&mut self, byte: u8, output: &mut Vec<u8>) {
        let previous = self.tag[self.tag.len() - 1];

        if self.tag.len() == 1 && !(is_name_char(byte) || matches!(byte, b'/' | b'!' | b'?')) {
            self.state = State::Text;
            output.push(b'<');
            self.process_text_byte(byte, output);
            return;
        }

        self.tag.push(byte);

        if self.tag == b"<!--" {
            self.state = State::Comment;
            self.comment_dashes = 0;
            return;
        }

        if self.quote != 0 {
            if byte == self.quote {
                self.quote = 0;
            }
        } else if matches!(byte, b'"' | b'\'') && previous == b'=' {
            self.quote = byte;
        } else if byte == b'>' {
            self.end_tag(output);
            return;
        }

        if self.tag.len() == MAX_TAG_LENGTH {
            self.flush_tag(output);
        }
    }

    fn process_suppressed_tag_byte(&mut self, byte: u8, output: &mut Vec<u8>) {
        if byte == b'<' {
            self.begin_tag(output);
            return;
        }

        self.tag.push(byte);

        if byte == b'>' {
            self.end_tag(output);
        } else if self.tag.len() == MAX_TAG_LENGTH {
            self.state = State::Text;
        }
    }

    fn process_comment_byte(&mut self, byte: u8) {
        if byte == b'>' && self.comment_dashes >= 2 {
            self.state = State::Text;
        }

        self.comment_dashes = if byte == b'-' {
            self.comment_dashes + 1
        } else {
            0
        };
    }

    fn flush_tag(&mut self, output: &mut Vec<u8>) {
        self.state = State::Text;

        if self.suppressed_element.is_none() {
            utf8::append_as_cp1252(output, &self.tag);
        }
    }

    fn end_tag(&mut self, output: &mut Vec<u8>) {
        self.state = State::Text;

        let tag = core::mem::take(&mut self.tag);
        self.rewrite_tag(&Tag::parse(&tag), output);
        self.tag = tag;
    }

    fn rewrite_tag(&mut self, tag: &Tag, output: &mut Vec<u8>) {
        if let Some(suppressed) = &self.suppressed_element {
            if tag.closing && tag.name.eq_ignore_ascii_case(suppressed) {
                self.suppressed_element = None;
            }
            return;
        }

        if tag.name.is_empty() || is_listed(tag.name, DROPPED_ELEMENTS) {
            return;
        }

        if is_listed(tag.name, SUPPRESSED_ELEMENTS) {
            if !tag.closing && !tag.self_closing {
                self.suppressed_element = Some(tag.name.to_ascii_lowercase());
            }
            return;
        }

        if tag.name.eq_ignore_ascii_case(b"img") {
            tag.append_image_replacement(output);
            return;
        }

        let name = if is_listed(tag.name, BLOCK_ELEMENTS) {
            b"div"
        } else {
            tag.name
        };

        output.push(b'<');
        if tag.closing {
            output.push(b'/');
        }
        output.extend_from_slice(name);
        if !tag.closing {
            tag.append_attributes(output);
        }
        output.push(b'>');

        if tag.closing && name.eq_ignore_ascii_case(b"span") {
            output.push(b' ');
        }
    }
}

impl<'a> Tag<'a> {
    fn parse(data: &'a [u8]) -> Self {
        let closing = data.get(1) == Some(&b'/');
        let name_start = if closing { 2 } else { 1 };
        let name_end = skip_name(data, name_start);

        Tag {
            data,
            closing,
            self_closing: data.len() >= 3 && data[data.len() - 2] == b'/',
            name: &data[name_start..name_end],
            attributes_offset: name_end,
        }
    }

    fn attributes(&self) -> Attributes<'a> {
        Attributes {
            data: &self.data[..self.data.len() - 1],
            offset: self.attributes_offset,
        }
    }

    fn find_attribute(&self, name: &[u8]) -> Option<Attribute<'a>> {
        self.attributes()
            .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
    }

    fn append_image_replacement(&self, output: &mut Vec<u8>) {
        if let Some(src) = self.find_attribute(b"src") {
            if is_svg_url(src.value) {
                if let Some(alt) = self.find_attribute(b"alt") {
                    utf8::append_as_cp1252(output, alt.value);
                }
                return;
            }
        }

        output.extend_from_slice(b"<img");
        self.append_attributes(output);
        output.push(b'>');
    }

    fn append_attributes(&self, output: &mut Vec<u8>) {
        for attribute in self
            .attributes()
            .filter(|attribute| is_listed(attribute.name, ALLOWED_ATTRIBUTES))
        {
            output.push(b' ');
            utf8::append_as_cp1252(output, attribute.raw);
        }
    }
}

impl<'a> Iterator for Attributes<'a> {
    type Item = Attribute<'a>;

    fn next(&mut self) -> Option<Attribute<'a>> {
        let data = self.data;
        let end = data.len();
        let mut i = self.offset;

        while i != end && (is_space(data[i]) || data[i] == b'/') {
            i += 1;
        }
        if i == end {
            return None;
        }

        let start = i;
        i = skip_name(data, i);
        let name = &data[start..i];
        if i == start {
            i += 1;
        }
        let mut value: &[u8] = &[];

        i = skip_spaces(data, i);
        if i != end && data[i] == b'=' {
            i = skip_spaces(data, i + 1);
            if i != end && matches!(data[i], b'"' | b'\'') {
                let quote = data[i];
                let value_start = i + 1;
                i = value_start;
                while i != end && data[i] != quote {
                    i += 1;
                }
                value = &data[value_start..i];
                if i != end {
                    i += 1;
                }
            } else {
                let value_start = i;
                while i != end && !is_space(data[i]) {
                    i += 1;
                }
                value = &data[value_start..i];
            }
        }

        self.offset = i;

        Some(Attribute {
            name,
            raw: &data[start..i],
            value,
        })
    }
}

fn is_svg_url(url: &[u8]) -> bool {
    let end = url
        .iter()
        .position(|&c| c == b'?' || c == b'#')
        .unwrap_or(url.len());
    end >= 4 && url[end - 4..end].eq_ignore_ascii_case(b".svg")
}

fn skip_name(data: &[u8], mut offset: usize) -> usize {
    while offset != data.len() && is_name_char(data[offset]) {
        offset += 1;
    }
    offset
}

fn skip_spaces(data: &[u8], mut offset: usize) -> usize {
    while offset != data.len() && is_space(data[offset]) {
        offset += 1;
    }
    offset
}

fn is_name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b':')
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\r' | b'\n' | b'\x0c')
}

fn is_listed(name: &[u8], list: &[&[u8]]) -> bool {
    list.iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
}
