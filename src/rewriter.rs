use crate::css::CssRewriter;
use crate::html::HtmlRewriter;

use alloc::vec::Vec;

pub enum Rewriter {
    Html(HtmlRewriter),
    Css(CssRewriter),
}

impl Rewriter {
    pub fn for_content_type(content_type: &[u8], viewport_width: i32) -> Option<Self> {
        let content_type = content_type.to_ascii_lowercase();
        if content_type.starts_with(b"text/html") {
            Some(Rewriter::Html(HtmlRewriter::new(viewport_width)))
        } else if content_type.starts_with(b"text/css") {
            Some(Rewriter::Css(CssRewriter::new(viewport_width)))
        } else {
            None
        }
    }

    pub fn replaces_cache_file(&self) -> bool {
        matches!(self, Rewriter::Css(_))
    }

    pub fn feed(&mut self, data: &[u8], output: &mut Vec<u8>) {
        match self {
            Rewriter::Html(rewriter) => rewriter.feed(data, output),
            Rewriter::Css(rewriter) => rewriter.feed(data, output),
        }
    }

    pub fn finish(&mut self, output: &mut Vec<u8>) {
        match self {
            Rewriter::Html(rewriter) => rewriter.finish(output),
            Rewriter::Css(rewriter) => rewriter.finish(output),
        }
    }
}
