use iefixup::html::HtmlRewriter;
use iefixup::rewriter::Rewriter;
use iefixup::stream::Stream;

const VIEWPORT_WIDTH: i32 = 640;

const PAGE: &str = r#"<!doctype html>
<html lang="nb">
<head>
<meta charset="utf-8">
<title>Main Functions — MBBS.no</title>
<link rel="stylesheet" href="/assets/site.css">
<script type="application/ld+json">{"a": "<b>"}</script>
<style>:root { --fg: rgba(0, 0, 255, 0.9); } nav > a { color: var(--fg); }</style>
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
    let expected = "\n<html lang=\"nb\">\n<head>\n<meta>\n<title>Main Functions - MBBS.no</title>\n<link rel=\"stylesheet\" href=\"/assets/site.css\">\n\n<style>div.ie-nav a { color: #0000ff }\n</style>\n</head>\n<body>\n<div class=\"ie-header site-header\"><div aria-label=\"x\" class=\"ie-nav\"><a href=\"/\" data-key=\"q\"><span>Q</span><span>Main</span></a></div></div>\n\n<div id=\"main\" class=\"ie-main\"><p>B\u{e5}tverksted i Tauv\u{e5}gen - \u{ab}velkommen\u{bb} if a < b</p>\nTau Slipp<img src=\"/2.jpg\" alt=\"B\u{e5}t\" style=\"object-position: 50%\">\n<pre>  keep   spacing </pre></div>\n</body>\n</html>\n";
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
        let mut stream = Stream::new(Some(Rewriter::Html(HtmlRewriter::new(VIEWPORT_WIDTH))));
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
    let mut rewriter = HtmlRewriter::new(VIEWPORT_WIDTH);
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

#[test]
fn translates_stylesheets() {
    let stylesheet = r#"@charset "utf-8";
:root { --bg: #f5f7f6; --accent: hsl(120, 100%, 25%); }
/* comment { } */
body { background: var(--bg); padding: 1.5rem calc(2rem + 4px) !important; }
.hero, header::before, .x[hidden] { background: linear-gradient(135deg, rgba(22, 56, 77, 0.96), transparent); }
.badge { color: var(--accent); border: 1px solid rgba(0, 0, 0, 0.1); width: min(1120px, calc(100% - 2rem)); }
.slide { inset: 0 2px; opacity: 0; }
@media (min-width: 600px) { .wide { margin: 0 auto; } }
@media (max-width: 600px), print { .narrow { display: none; } }
@font-face { font-family: X; src: url(x.woff2); }
"#;
    let expected = "body { background: #f5f7f6; padding: 24px 36px !important }\n\
.hero { background-color: #16384d }\n\
.badge { color: #007f00; width: 608px }\n\
.slide { top: 0; right: 2px; bottom: 0; left: 2px; visibility: hidden }\n\
.wide { margin: 0 auto }\n";

    let mut rewriter = Rewriter::for_content_type(b"text/css", VIEWPORT_WIDTH).unwrap();
    let mut output = Vec::new();
    for chunk in stylesheet.as_bytes().chunks(5) {
        rewriter.feed(chunk, &mut output);
    }
    rewriter.finish(&mut output);
    assert_eq!(String::from_utf8(output).unwrap(), expected);
}

#[test]
fn approximates_layout() {
    let stylesheet = ".row { display: flex; gap: 4px 12px; }
.column { display: flex; flex-direction: column; gap: 8px; }
.grid { display: grid; }
.page { width: 600px; margin: 10px auto 0; }
.narrow { max-width: 400px; margin: 0 auto; }
@media (max-width: 700px) { .page { width: 500px; margin-top: 4px; } }
.rule { height: 1px; }
.frame { width: 50%; aspect-ratio: 4 / 3; }
.slide { position: absolute; inset: 0; }
";
    let expected = ".row { display: block; gap: 4px 12px }
.row * { margin-right: 12px }
.column { display: block; flex-direction: column; gap: 8px }
.grid { display: block }
.page { width: 600px; margin: 10px 20px 0 }
.narrow { max-width: 400px; margin: 0 120px }
.page { width: 500px; margin-top: 4px; margin-left: 70px; margin-right: 70px }
.rule { height: 1px; overflow: hidden }
.frame { width: 50%; height: 240px }
.slide { position: absolute; top: 0; right: 0; bottom: 0; left: 0; height: 100%; width: 100% }
";

    let mut rewriter = Rewriter::for_content_type(b"text/css", VIEWPORT_WIDTH).unwrap();
    let mut output = Vec::new();
    rewriter.feed(stylesheet.as_bytes(), &mut output);
    rewriter.finish(&mut output);
    assert_eq!(String::from_utf8(output).unwrap(), expected);
}
