use crate::html::HtmlRewriter;

use alloc::vec::Vec;

pub struct Stream {
    rewriter: Option<HtmlRewriter>,
    pending: Vec<u8>,
    finished: bool,
}

impl Stream {
    pub fn new(rewriter: Option<HtmlRewriter>) -> Self {
        Stream {
            rewriter,
            pending: Vec::new(),
            finished: false,
        }
    }

    pub fn is_rewriting(&self) -> bool {
        self.rewriter.is_some()
    }

    pub fn has_pending_output(&self) -> bool {
        !self.pending.is_empty() || self.finished
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn feed(&mut self, data: &[u8]) {
        let rewriter = self.rewriter.as_mut().unwrap();

        if data.is_empty() {
            rewriter.finish(&mut self.pending);
            self.finished = true;
            return;
        }

        let start = self.pending.len();
        rewriter.feed(data, &mut self.pending);

        let produced = self.pending.len() - start;
        if produced < data.len() {
            let offset = find_padding_offset(&self.pending, start);
            self.pending.splice(
                offset..offset,
                core::iter::repeat_n(b' ', data.len() - produced),
            );
        }
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> usize {
        let n = self.pending.len().min(buffer.len());
        buffer[..n].copy_from_slice(&self.pending[..n]);
        self.pending.drain(..n);
        n
    }
}

fn find_padding_offset(output: &[u8], start: usize) -> usize {
    let chunk = &output[start..];
    if let Some(index) = chunk.iter().rposition(|&c| c == b'>') {
        return start + index;
    }
    match chunk.iter().rposition(|&c| c == b' ' || c == b'\n') {
        Some(index) => start + index + 1,
        None => output.len(),
    }
}
