use crate::css::CssRewriter;
use crate::utf8::{self, Utf8Decoder};

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

const MAX_TAG_LENGTH: usize = 65536;

const SUPPRESSED_ELEMENTS: &[&[u8]] = &[b"script", b"template", b"svg", b"style"];

const DROPPED_ELEMENTS: &[&[u8]] = &[
    b"noscript",
    b"picture",
    b"source",
    b"track",
    b"video",
    b"audio",
    b"canvas",
];

const RENAMED_ELEMENTS: &[&[u8]] = &[
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

const STYLE_ELEMENT: &[u8] = b"style";
const RENAMED_ELEMENT_CLASS_PREFIX: &str = "ie-";

pub struct HtmlRewriter {
    state: State,
    text_decoder: Utf8Decoder,
    tag: Vec<u8>,
    quote: u8,
    comment_dashes: u32,
    suppressed_element: Option<Vec<u8>>,
    css: CssRewriter,
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
    pub fn new(viewport_width: i32) -> Self {
        HtmlRewriter {
            state: State::default(),
            text_decoder: Utf8Decoder::default(),
            tag: Vec::new(),
            quote: 0,
            comment_dashes: 0,
            suppressed_element: None,
            css: CssRewriter::new(viewport_width),
        }
    }

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
        } else if self.is_in_style() {
            self.feed_style(&[byte], output);
        }
    }

    fn is_in_style(&self) -> bool {
        self.suppressed_element.as_deref() == Some(STYLE_ELEMENT)
    }

    fn feed_style(&mut self, data: &[u8], output: &mut Vec<u8>) {
        let mut translated = Vec::new();
        self.css.feed(data, &mut translated);
        utf8::append_as_cp1252(output, &translated);
    }

    fn finish_style(&mut self, output: &mut Vec<u8>) {
        let mut translated = Vec::new();
        self.css.finish(&mut translated);
        utf8::append_as_cp1252(output, &translated);
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
            let ends_suppression = tag.closing && tag.name.eq_ignore_ascii_case(suppressed);
            match (ends_suppression, self.is_in_style()) {
                (true, true) => {
                    self.finish_style(output);
                    output.extend_from_slice(b"</style>");
                    self.suppressed_element = None;
                }
                (true, false) => self.suppressed_element = None,
                (false, true) => self.feed_style(tag.data, output),
                (false, false) => {}
            }
            return;
        }

        if tag.name.is_empty() || is_listed(tag.name, DROPPED_ELEMENTS) {
            return;
        }

        if is_listed(tag.name, SUPPRESSED_ELEMENTS) {
            if !tag.closing && !tag.self_closing {
                self.suppressed_element = Some(tag.name.to_ascii_lowercase());
                if self.is_in_style() {
                    output.extend_from_slice(b"<style>");
                }
            }
            return;
        }

        if tag.name.eq_ignore_ascii_case(b"img") {
            tag.append_image_replacement(&self.css, output);
            return;
        }

        let renamed = is_renamed_element(tag.name);
        let name = if renamed { b"div" } else { tag.name };

        output.push(b'<');
        if tag.closing {
            output.push(b'/');
        }
        output.extend_from_slice(name);
        if !tag.closing {
            let element_class =
                renamed.then(|| renamed_element_class(&String::from_utf8_lossy(tag.name)));
            tag.append_attributes(element_class.as_deref(), &self.css, output);
        }
        output.push(b'>');
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

    fn append_image_replacement(&self, css: &CssRewriter, output: &mut Vec<u8>) {
        if let Some(src) = self.find_attribute(b"src") {
            if is_svg_url(src.value) {
                if let Some(alt) = self.find_attribute(b"alt") {
                    utf8::append_as_cp1252(output, alt.value);
                }
                return;
            }
        }

        output.extend_from_slice(b"<img");
        self.append_attributes(None, css, output);
        output.push(b'>');
    }

    fn append_attributes(
        &self,
        element_class: Option<&str>,
        css: &CssRewriter,
        output: &mut Vec<u8>,
    ) {
        let mut pending_class = element_class;

        for attribute in self
            .attributes()
            .filter(|attribute| !attribute.name.eq_ignore_ascii_case(b"charset"))
        {
            if attribute.name.eq_ignore_ascii_case(b"style") {
                append_style_attribute(attribute.value, css, output);
            } else if let Some(class) =
                pending_class.filter(|_| attribute.name.eq_ignore_ascii_case(b"class"))
            {
                append_class_attribute(class, attribute.value, output);
                pending_class = None;
            } else {
                output.push(b' ');
                utf8::append_as_cp1252(output, attribute.raw);
            }
        }

        if let Some(class) = pending_class {
            append_class_attribute(class, b"", output);
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

fn append_style_attribute(value: &[u8], css: &CssRewriter, output: &mut Vec<u8>) {
    let declarations = css.translate_declarations(&String::from_utf8_lossy(value));
    if declarations.is_empty() {
        return;
    }
    output.extend_from_slice(b" style=\"");
    utf8::append_as_cp1252(output, declarations.replace('"', "'").as_bytes());
    output.push(b'"');
}

fn append_class_attribute(element_class: &str, classes: &[u8], output: &mut Vec<u8>) {
    output.extend_from_slice(b" class=\"");
    output.extend_from_slice(element_class.as_bytes());
    if !classes.is_empty() {
        output.push(b' ');
        utf8::append_as_cp1252(output, classes);
    }
    output.push(b'"');
}

pub fn is_renamed_element(name: &[u8]) -> bool {
    is_listed(name, RENAMED_ELEMENTS)
}

pub fn renamed_element_class(name: &str) -> String {
    format!(
        "{RENAMED_ELEMENT_CLASS_PREFIX}{}",
        name.to_ascii_lowercase()
    )
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
