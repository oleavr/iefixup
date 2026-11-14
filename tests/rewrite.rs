use iefixup::html::HtmlRewriter;
use iefixup::stream::Stream;

const PAGE: &str = r#"<!doctype html>
<html lang="nb">
<head>
<meta charset="utf-8">
<title>Main Functions — MBBS.no</title>
<link rel="stylesheet" href="/assets/site.css">
<script type="application/ld+json">{"a": "<b>"}</script>
</head>
<body>
<header class="site-header"><nav aria-label="x"><a href="/" data-key="q"><span>Q</span><span>Main</span></a></nav></header>
<!-- a comment with <tags> -->
<main id="main"><p>Båtverksted i Tauvågen – «velkommen» if a < b</p>
<img src="/logo.svg?v=2" alt="Tau Slipp" class="x"><img src="/2.jpg" alt="Båt" style="object-position: 50%">
<script>if (a<b) { x = "</div>"; }</script><pre>  keep   spacing </pre></main>
</body>
</html>
"#;

#[test]
fn rewrites_modern_markup() {
    let output = rewrite_in_chunks(PAGE.as_bytes(), usize::MAX);
    let expected = "\n<html>\n<head>\n<meta>\n<title>Main Functions - MBBS.no</title>\n\n\n</head>\n<body>\n<div><div><a href=\"/\"><span>Q</span> <span>Main</span> </a></div></div>\n\n<div><p>B\u{e5}tverksted i Tauv\u{e5}gen - \u{ab}velkommen\u{bb} if a < b</p>\nTau Slipp<img src=\"/2.jpg\" alt=\"B\u{e5}t\">\n<pre>  keep   spacing </pre></div>\n</body>\n</html>\n";
    assert_eq!(output, cp1252(expected));
}

#[test]
fn output_does_not_depend_on_chunking() {
    let reference = rewrite_in_chunks(PAGE.as_bytes(), usize::MAX);
    for chunk_size in 1..64 {
        assert_eq!(
            rewrite_in_chunks(PAGE.as_bytes(), chunk_size),
            reference,
            "chunk size {chunk_size}"
        );
    }
}

#[test]
fn stream_preserves_chunk_lengths() {
    for chunk_size in [1, 7, 33, 100, 4096] {
        let mut stream = Stream::new(Some(HtmlRewriter::default()));
        let mut buffer = vec![0u8; 8192];
        let mut total = 0;
        for chunk in PAGE.as_bytes().chunks(chunk_size) {
            stream.feed(chunk);
            assert_eq!(
                stream.read(&mut buffer[..chunk.len()]),
                chunk.len(),
                "chunk size {chunk_size}"
            );
            total += chunk.len();
        }
        stream.feed(&[]);
        while stream.read(&mut buffer) != 0 {}
        assert_eq!(total, PAGE.len());
    }
}

fn rewrite_in_chunks(input: &[u8], chunk_size: usize) -> Vec<u8> {
    let mut rewriter = HtmlRewriter::default();
    let mut output = Vec::new();
    for chunk in input.chunks(chunk_size.min(input.len())) {
        rewriter.feed(chunk, &mut output);
    }
    rewriter.finish(&mut output);
    output
}

fn cp1252(text: &str) -> Vec<u8> {
    text.chars().map(|c| c as u32 as u8).collect()
}
