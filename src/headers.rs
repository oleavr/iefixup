use alloc::vec::Vec;

const ACCEPT_ENCODING: &[u8] = b"accept-encoding:";

pub fn without_accept_encoding(headers: &[u8]) -> Option<Vec<u8>> {
    let mut filtered = Vec::with_capacity(headers.len());
    let mut removed = false;

    for line in headers.split_inclusive(|&c| c == b'\n') {
        if line.len() >= ACCEPT_ENCODING.len()
            && line[..ACCEPT_ENCODING.len()].eq_ignore_ascii_case(ACCEPT_ENCODING)
        {
            removed = true;
        } else {
            filtered.extend_from_slice(line);
        }
    }

    removed.then_some(filtered)
}
