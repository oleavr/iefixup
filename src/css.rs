use crate::selector;
use crate::style::StyleContext;
use crate::value;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

pub struct CssRewriter {
    scanner: RuleScanner,
    context: StyleContext,
}

#[derive(Default)]
struct RuleScanner {
    rule: Vec<u8>,
    depth: u32,
    quote: u8,
    in_comment: bool,
    comment_star: bool,
}

impl CssRewriter {
    pub fn new(viewport_width: i32) -> Self {
        CssRewriter {
            scanner: RuleScanner::default(),
            context: StyleContext::new(viewport_width),
        }
    }

    pub fn feed(&mut self, data: &[u8], output: &mut Vec<u8>) {
        for &byte in data {
            if let Some(rule) = self.scanner.push(byte) {
                translate_rule(&rule, &mut self.context, output);
            }
        }
    }

    pub fn finish(&mut self, output: &mut Vec<u8>) {
        let rule = self.scanner.take();
        translate_rule(&rule, &mut self.context, output);
    }

    pub fn translate_declarations(&self, block: &str) -> String {
        value::translate_declarations(block, &self.context, false).declarations
    }
}

impl RuleScanner {
    fn push(&mut self, byte: u8) -> Option<String> {
        if self.in_comment {
            self.in_comment = !(byte == b'/' && self.comment_star);
            self.comment_star = byte == b'*';
            return None;
        }

        if self.quote != 0 {
            if byte == self.quote {
                self.quote = 0;
            }
            self.rule.push(byte);
            return None;
        }

        match byte {
            b'*' if self.rule.last() == Some(&b'/') => {
                self.rule.pop();
                self.in_comment = true;
                self.comment_star = false;
                return None;
            }
            b'"' | b'\'' => self.quote = byte,
            b'{' => self.depth += 1,
            b'}' if self.depth != 0 => self.depth -= 1,
            _ => {}
        }
        self.rule.push(byte);

        let completed = self.depth == 0 && matches!(byte, b'}' | b';');
        completed.then(|| self.take())
    }

    fn take(&mut self) -> String {
        let rule = String::from_utf8_lossy(&self.rule).into_owned();
        self.rule.clear();
        rule
    }
}

fn translate_rule(rule: &str, context: &mut StyleContext, output: &mut Vec<u8>) {
    let rule = rule.trim();
    let Some(open) = rule.find('{') else {
        translate_statement(rule, output);
        return;
    };
    let prelude = rule[..open].trim();
    let body = &rule[open + 1..];
    let body = body.strip_suffix('}').unwrap_or(body);

    match prelude.strip_prefix('@') {
        Some(at_rule) => translate_at_rule(at_rule, body, context, output),
        None => translate_style_rule(prelude, body, context, output),
    }
}

fn translate_statement(statement: &str, output: &mut Vec<u8>) {
    if statement.starts_with("@import") {
        output.extend_from_slice(statement.as_bytes());
        output.push(b'\n');
    }
}

fn translate_at_rule(at_rule: &str, body: &str, context: &mut StyleContext, output: &mut Vec<u8>) {
    let keyword_end = at_rule
        .find(|c: char| c == '(' || c.is_ascii_whitespace())
        .unwrap_or(at_rule.len());
    let (keyword, condition) = at_rule.split_at(keyword_end);

    if keyword.eq_ignore_ascii_case("media") && media_matches(condition, context) {
        translate_stylesheet(body, context, output);
    }
}

fn translate_stylesheet(text: &str, context: &mut StyleContext, output: &mut Vec<u8>) {
    let mut scanner = RuleScanner::default();
    for &byte in text.as_bytes() {
        if let Some(rule) = scanner.push(byte) {
            translate_rule(&rule, context, output);
        }
    }
    let rule = scanner.take();
    translate_rule(&rule, context, output);
}

fn translate_style_rule(
    prelude: &str,
    body: &str,
    context: &mut StyleContext,
    output: &mut Vec<u8>,
) {
    if selector::selects_root(prelude) {
        value::record_custom_properties(body, context);
    }

    let selectors = selector::translate_selector_list(prelude);
    if selectors.is_empty() {
        return;
    }

    let selector_list = selectors.join(", ");
    let translation =
        value::translate_declarations(body, context, context.is_centered(&selector_list));
    context.set_centered(&selector_list, translation.centered);
    if !translation.declarations.is_empty() {
        write_rule(&selector_list, &translation.declarations, output);
    }

    if let Some(spacing) = value::flex_item_spacing(body, context) {
        let descendants: Vec<String> = selectors
            .iter()
            .map(|selector| format!("{selector} *"))
            .collect();
        write_rule(
            &descendants.join(", "),
            &format!("margin-right: {spacing}"),
            output,
        );
    }
}

fn write_rule(selectors: &str, declarations: &str, output: &mut Vec<u8>) {
    output.extend_from_slice(selectors.as_bytes());
    output.extend_from_slice(b" { ");
    output.extend_from_slice(declarations.as_bytes());
    output.extend_from_slice(b" }\n");
}

fn media_matches(condition: &str, context: &StyleContext) -> bool {
    value::split_top_level(condition, b',')
        .into_iter()
        .any(|query| media_query_matches(query, context))
}

fn media_query_matches(query: &str, context: &StyleContext) -> bool {
    let lowercase = query.to_ascii_lowercase();
    let (negated, query) = match lowercase.strip_prefix("not ") {
        Some(query) => (true, query),
        None => (false, lowercase.strip_prefix("only ").unwrap_or(&lowercase)),
    };

    let matches = query
        .split(" and ")
        .all(|part| media_part_matches(part.trim(), context));
    matches != negated
}

fn media_part_matches(part: &str, context: &StyleContext) -> bool {
    let Some(feature) = part
        .strip_prefix('(')
        .and_then(|part| part.strip_suffix(')'))
    else {
        return matches!(part, "" | "all" | "screen");
    };
    let Some((name, value)) = feature.split_once(':') else {
        return false;
    };

    let viewport = context.viewport_width();
    let length = || value::length_in_px(value, context);
    match name.trim() {
        "min-width" => length().is_some_and(|width| viewport >= width),
        "max-width" => length().is_some_and(|width| viewport <= width),
        "prefers-color-scheme" => value.trim() == "light",
        _ => false,
    }
}
