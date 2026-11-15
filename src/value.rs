use crate::style::StyleContext;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

const ROOT_FONT_SIZE: i32 = 16;
const MAX_VAR_DEPTH: u32 = 8;
const MIN_OPAQUE_ALPHA: i32 = 50;
const IMPORTANT: &str = "!important";

const NAMED_COLORS: &[&str] = &[
    "black", "silver", "gray", "white", "maroon", "red", "purple", "fuchsia", "green", "lime",
    "olive", "yellow", "navy", "blue", "teal", "aqua", "orange",
];

struct Declaration {
    name: String,
    value: String,
    priority: &'static str,
}

pub struct Translation {
    pub declarations: String,
    pub centered: bool,
}

pub fn translate_declarations(
    block: &str,
    context: &StyleContext,
    centered_before: bool,
) -> Translation {
    let mut declarations: Vec<Declaration> = declarations(block)
        .filter_map(|(name, value)| translate_declaration(&name, value, context))
        .flatten()
        .collect();

    let centered = center_horizontally(&mut declarations, context, centered_before);
    size_aspect_ratio_boxes(&mut declarations, context);
    stretch_absolute_boxes(&mut declarations);
    clip_short_boxes(&mut declarations, context);

    let declarations = declarations
        .iter()
        .map(|declaration| {
            format!(
                "{}: {}{}",
                declaration.name, declaration.value, declaration.priority
            )
        })
        .collect::<Vec<_>>()
        .join("; ");

    Translation {
        declarations,
        centered,
    }
}

pub fn flex_item_spacing(block: &str, context: &StyleContext) -> Option<String> {
    let mut is_flex_row = false;
    let mut spacing = None;

    for (name, value) in declarations(block) {
        let value = resolve_vars(value.trim_end_matches(IMPORTANT).trim(), context, 0)?;
        match name.as_str() {
            "display" => is_flex_row = matches!(value.as_str(), "flex" | "inline-flex"),
            "flex-direction" if value.starts_with("column") => return None,
            "gap" => spacing = components(&value).last().map(String::from),
            "column-gap" => spacing = Some(value),
            _ => {}
        }
    }

    let spacing = translate_value(&spacing.filter(|_| is_flex_row)?, context)?;
    (spacing != "0").then_some(spacing)
}
pub fn record_custom_properties(block: &str, context: &mut StyleContext) {
    for declaration in split_top_level(block, b';') {
        if let Some((name, value)) = declaration.split_once(':') {
            let name = name.trim();
            if name.starts_with("--") {
                context.set_custom_property(name, value.trim());
            }
        }
    }
}

pub fn length_in_px(value: &str, context: &StyleContext) -> Option<i32> {
    evaluate_term(value.trim(), context).map(|hundredths| hundredths / 100)
}

fn declarations(block: &str) -> impl Iterator<Item = (String, &str)> {
    split_top_level(block, b';')
        .into_iter()
        .filter_map(|declaration| {
            let (name, value) = declaration.split_once(':')?;
            let name = name.trim();
            (!name.starts_with("--")).then(|| (name.to_ascii_lowercase(), value.trim()))
        })
}

fn translate_declaration(
    name: &str,
    value: &str,
    context: &StyleContext,
) -> Option<Vec<Declaration>> {
    let (value, priority) = match value.strip_suffix(IMPORTANT) {
        Some(value) => (value.trim_end(), " !important"),
        None => (value, ""),
    };
    let value = resolve_vars(value, context, 0)?;
    let declaration = |name: &str, value: String| Declaration {
        name: String::from(name),
        value,
        priority,
    };

    if name.starts_with("background") && value.contains("gradient(") {
        return Some(alloc::vec![declaration(
            "background-color",
            find_color(&value)?
        )]);
    }

    let value = translate_value(&value, context)?;
    let declarations = match name {
        "inset" => translate_inset(&value)
            .into_iter()
            .map(|(name, offset)| declaration(name, String::from(offset)))
            .collect(),
        "opacity" => alloc::vec![declaration(
            "visibility",
            String::from(translate_opacity(&value)?)
        )],
        "display" => alloc::vec![declaration(
            "display",
            String::from(translate_display(&value))
        )],
        _ => alloc::vec![declaration(name, value)],
    };
    Some(declarations)
}

fn clip_short_boxes(declarations: &mut Vec<Declaration>, context: &StyleContext) {
    let is_short = declarations.iter().any(|declaration| {
        declaration.name == "height"
            && evaluate_term(&declaration.value, context)
                .is_some_and(|height| height < ROOT_FONT_SIZE * 100)
    });
    let has_overflow = declarations
        .iter()
        .any(|declaration| declaration.name.starts_with("overflow"));
    if is_short && !has_overflow {
        declarations.push(Declaration {
            name: String::from("overflow"),
            value: String::from("hidden"),
            priority: "",
        });
    }
}

fn center_horizontally(
    declarations: &mut Vec<Declaration>,
    context: &StyleContext,
    centered_before: bool,
) -> bool {
    let has_margin = declarations
        .iter()
        .any(|declaration| is_horizontal_margin(&declaration.name));
    let has_auto_margin = declarations.iter().any(|declaration| {
        is_horizontal_margin(&declaration.name)
            && components(&declaration.value).any(|side| side == "auto")
    });
    if !has_auto_margin && (has_margin || !centered_before) {
        return false;
    }

    let px = |name: &str| declared_px(declarations, name, context);
    let viewport = context.viewport_width() * 100;
    let Some(width) = px("width").or_else(|| px("max-width").map(|width| width.min(viewport)))
    else {
        return true;
    };
    let offset = format_px((viewport - width).max(0) / 2);

    if !has_margin {
        for name in ["margin-left", "margin-right"] {
            declarations.push(Declaration {
                name: String::from(name),
                value: offset.clone(),
                priority: "",
            });
        }
        return true;
    }

    for declaration in declarations.iter_mut() {
        match declaration.name.as_str() {
            "margin" => declaration.value = center_margin(&declaration.value, &offset),
            "margin-left" | "margin-right" if declaration.value == "auto" => {
                declaration.value = offset.clone()
            }
            _ => {}
        }
    }
    true
}

fn size_aspect_ratio_boxes(declarations: &mut Vec<Declaration>, context: &StyleContext) {
    let Some(index) = declarations
        .iter()
        .position(|declaration| declaration.name == "aspect-ratio")
    else {
        return;
    };
    let aspect_ratio = declarations.remove(index);
    if has_declaration(declarations, "height") {
        return;
    }
    let Some((ratio_width, ratio_height)) = parse_aspect_ratio(&aspect_ratio.value) else {
        return;
    };

    let width = declared_px(declarations, "width", context)
        .unwrap_or(context.viewport_width() * 100);
    declarations.push(Declaration {
        name: String::from("height"),
        value: format_px(width * ratio_height / ratio_width),
        priority: aspect_ratio.priority,
    });
}

fn stretch_absolute_boxes(declarations: &mut Vec<Declaration>) {
    let is_absolute = declarations.iter().any(|declaration| {
        declaration.name == "position" && declaration.value == "absolute"
    });
    if !is_absolute {
        return;
    }

    for (size, start, end) in [("height", "top", "bottom"), ("width", "left", "right")] {
        let is_pinned = |name: &str| {
            declarations
                .iter()
                .any(|declaration| declaration.name == name && declaration.value == "0")
        };
        if is_pinned(start) && is_pinned(end) && !has_declaration(declarations, size) {
            declarations.push(Declaration {
                name: String::from(size),
                value: String::from("100%"),
                priority: "",
            });
        }
    }
}

fn parse_aspect_ratio(value: &str) -> Option<(i32, i32)> {
    let ratio: Vec<&str> = components(value).collect();
    match ratio.as_slice() {
        [width] => Some((parse_number(width)?.0, 100)),
        [width, height] => Some((parse_number(width)?.0, parse_number(height)?.0)),
        _ => None,
    }
    .filter(|&(width, height)| width > 0 && height > 0)
}

fn has_declaration(declarations: &[Declaration], name: &str) -> bool {
    declarations.iter().any(|declaration| declaration.name == name)
}

fn declared_px(declarations: &[Declaration], name: &str, context: &StyleContext) -> Option<i32> {
    declarations
        .iter()
        .rev()
        .find(|declaration| declaration.name == name)
        .and_then(|declaration| evaluate_term(&declaration.value, context))
}

fn is_horizontal_margin(name: &str) -> bool {
    matches!(name, "margin" | "margin-left" | "margin-right")
}

fn center_margin(margin: &str, offset: &str) -> String {
    let sides: Vec<&str> = components(margin).collect();
    let is_horizontal = |index: usize| index % 2 == 1;
    match sides.as_slice() {
        ["auto"] => format!("0 {offset}"),
        _ => sides
            .iter()
            .enumerate()
            .map(|(index, side)| match *side {
                "auto" if is_horizontal(index) => offset,
                "auto" => "0",
                side => side,
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn translate_display(value: &str) -> &str {
    match value {
        "flex" | "grid" => "block",
        "inline-flex" | "inline-grid" => "inline",
        value => value,
    }
}

fn translate_inset(value: &str) -> [(&'static str, &str); 4] {
    let offsets: Vec<&str> = components(value).collect();
    let offset = |index: usize| offsets[index % offsets.len()];
    let (top, right) = (offset(0), offset(1));
    let bottom = offsets.get(2).copied().unwrap_or(top);
    let left = offsets.get(3).copied().unwrap_or(right);
    [
        ("top", top),
        ("right", right),
        ("bottom", bottom),
        ("left", left),
    ]
}

fn translate_opacity(value: &str) -> Option<&'static str> {
    let opacity = parse_alpha(value)?;
    Some(if opacity < MIN_OPAQUE_ALPHA {
        "hidden"
    } else {
        "visible"
    })
}

fn resolve_vars(value: &str, context: &StyleContext, depth: u32) -> Option<String> {
    let Some(start) = value.find("var(") else {
        return Some(String::from(value));
    };
    if depth == MAX_VAR_DEPTH {
        return None;
    }

    let arguments_start = start + 4;
    let end = arguments_start + matching_paren(&value[arguments_start..])?;
    let arguments = &value[arguments_start..end];
    let (name, fallback) = match arguments.split_once(',') {
        Some((name, fallback)) => (name.trim(), Some(fallback.trim())),
        None => (arguments.trim(), None),
    };
    let replacement = context.custom_property(name).or(fallback)?;

    let resolved = format!("{}{}{}", &value[..start], replacement, &value[end + 1..]);
    resolve_vars(&resolved, context, depth + 1)
}

fn find_color(value: &str) -> Option<String> {
    components(value).find_map(|component| match split_function(component) {
        Some((name, arguments)) if !is_color_function(name) => find_color(arguments),
        _ => parse_color(component),
    })
}

fn translate_value(value: &str, context: &StyleContext) -> Option<String> {
    let mut translated = String::new();
    let mut rest = value;

    while let Some(first) = rest.chars().next() {
        let end = component_end(rest);
        if end == 0 {
            translated.push(first);
            rest = &rest[first.len_utf8()..];
            continue;
        }
        translated.push_str(&translate_component(&rest[..end], context)?);
        rest = &rest[end..];
    }

    Some(translated)
}

fn translate_component(component: &str, context: &StyleContext) -> Option<String> {
    if component.starts_with('#') {
        return parse_color(component);
    }

    if let Some((name, arguments)) = split_function(component) {
        let name = name.to_ascii_lowercase();
        if is_color_function(&name) {
            return parse_color(component);
        }
        if is_math_function(&name) {
            return Some(format_px(evaluate_function(&name, arguments, context)?));
        }
        return Some(String::from(component));
    }

    match parse_number(component) {
        Some((_, unit)) if needs_absolute_length(unit) => {
            Some(format_px(evaluate_term(component, context)?))
        }
        _ => Some(String::from(component)),
    }
}

fn parse_color(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase();

    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    if let Some((name, arguments)) = split_function(&value) {
        return match name {
            "rgb" | "rgba" => parse_rgb(arguments),
            "hsl" | "hsla" => parse_hsl(arguments),
            _ => None,
        };
    }

    NAMED_COLORS.contains(&value.as_str()).then_some(value)
}

fn parse_hex_color(hex: &str) -> Option<String> {
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let digits: Vec<u8> = match hex.len() {
        3 | 4 => hex.bytes().flat_map(|digit| [digit, digit]).collect(),
        6 | 8 => hex.bytes().collect(),
        _ => return None,
    };
    if digits.len() == 8 {
        let alpha = u8::from_str_radix(core::str::from_utf8(&digits[6..]).ok()?, 16).ok()?;
        if i32::from(alpha) * 100 / 255 < MIN_OPAQUE_ALPHA {
            return None;
        }
    }
    Some(format!("#{}", core::str::from_utf8(&digits[..6]).ok()?))
}

fn parse_rgb(arguments: &str) -> Option<String> {
    let components = color_components(arguments)?;
    let mut rgb = [0i32; 3];
    for (channel, component) in rgb.iter_mut().zip(&components) {
        let (number, unit) = parse_number(component)?;
        *channel = match unit {
            "%" => number * 255 / 10000,
            "" => number / 100,
            _ => return None,
        };
    }
    Some(format_rgb(rgb))
}

fn parse_hsl(arguments: &str) -> Option<String> {
    let components = color_components(arguments)?;
    let (hue, hue_unit) = parse_number(components[0])?;
    let hue = match hue_unit {
        "" | "deg" => hue / 100,
        "turn" => hue * 360 / 100,
        _ => return None,
    }
    .rem_euclid(360);
    let saturation = parse_number(components[1])?.0 / 100;
    let lightness = parse_number(components[2])?.0 / 100;

    let chroma = (100 - (2 * lightness - 100).abs()) * saturation / 100;
    let x = chroma * (60 - ((hue % 120) - 60).abs()) / 60;
    let m = lightness - chroma / 2;
    let (r, g, b) = match hue / 60 {
        0 => (chroma, x, 0),
        1 => (x, chroma, 0),
        2 => (0, chroma, x),
        3 => (0, x, chroma),
        4 => (x, 0, chroma),
        _ => (chroma, 0, x),
    };
    Some(format_rgb([
        (r + m) * 255 / 100,
        (g + m) * 255 / 100,
        (b + m) * 255 / 100,
    ]))
}

fn color_components(arguments: &str) -> Option<Vec<&str>> {
    let components: Vec<&str> = arguments
        .split(|c: char| c == ',' || c == '/' || c.is_ascii_whitespace())
        .filter(|c| !c.is_empty())
        .collect();
    match components.len() {
        3 => Some(components),
        4 if parse_alpha(components[3])? >= MIN_OPAQUE_ALPHA => Some(components),
        _ => None,
    }
}

fn parse_alpha(component: &str) -> Option<i32> {
    match parse_number(component)? {
        (number, "") => Some(number),
        (number, "%") => Some(number / 100),
        _ => None,
    }
}

fn format_rgb(rgb: [i32; 3]) -> String {
    let [r, g, b] = rgb.map(|channel| channel.clamp(0, 255));
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn is_color_function(name: &str) -> bool {
    matches!(name, "rgb" | "rgba" | "hsl" | "hsla")
}

fn is_math_function(name: &str) -> bool {
    matches!(name, "calc" | "min" | "max" | "clamp")
}

fn needs_absolute_length(unit: &str) -> bool {
    matches!(
        unit.to_ascii_lowercase().as_str(),
        "rem" | "vw" | "dvw" | "svw" | "lvw" | "vh" | "dvh" | "svh" | "lvh"
    )
}

fn evaluate_function(name: &str, arguments: &str, context: &StyleContext) -> Option<i32> {
    let mut values = split_top_level(arguments, b',')
        .into_iter()
        .map(|argument| evaluate_expression(argument, context));
    match name {
        "calc" => evaluate_expression(arguments, context),
        "min" => values.try_fold(i32::MAX, |acc, value| Some(acc.min(value?))),
        "max" => values.try_fold(i32::MIN, |acc, value| Some(acc.max(value?))),
        _ => {
            let (low, preferred, high) = (values.next()??, values.next()??, values.next()??);
            Some(preferred.min(high).max(low))
        }
    }
}

fn evaluate_expression(expression: &str, context: &StyleContext) -> Option<i32> {
    let terms: Vec<&str> = components(expression).collect();
    let mut result = evaluate_term(terms.first()?, context)?;

    for pair in terms[1..].chunks(2) {
        let [operator, operand] = pair else {
            return None;
        };
        result = match *operator {
            "+" => result + evaluate_term(operand, context)?,
            "-" => result - evaluate_term(operand, context)?,
            "*" => result * parse_number(operand)?.0 / 100,
            _ => return None,
        };
    }

    Some(result)
}

fn evaluate_term(term: &str, context: &StyleContext) -> Option<i32> {
    if let Some((name, arguments)) = split_function(term) {
        return evaluate_function(&name.to_ascii_lowercase(), arguments, context);
    }
    if let Some(inner) = term
        .strip_prefix('(')
        .and_then(|term| term.strip_suffix(')'))
    {
        return evaluate_expression(inner, context);
    }

    let (number, unit) = parse_number(term)?;
    let viewport = context.viewport_width();
    let px = match unit.to_ascii_lowercase().as_str() {
        "" if number == 0 => 0,
        "px" => number,
        "pt" => number * 4 / 3,
        "em" | "rem" => number * ROOT_FONT_SIZE,
        "%" | "vw" | "dvw" | "svw" | "lvw" => number * viewport / 100,
        "vh" | "dvh" | "svh" | "lvh" => number * viewport * 3 / 400,
        _ => return None,
    };
    Some(px)
}

fn format_px(hundredths: i32) -> String {
    let sign = if hundredths < 0 { "-" } else { "" };
    let px = hundredths.abs() / 100;
    if px == 0 {
        String::from("0")
    } else {
        format!("{sign}{px}px")
    }
}

fn parse_number(value: &str) -> Option<(i32, &str)> {
    let bytes = value.as_bytes();
    let mut i = 0;
    let negative = bytes.first() == Some(&b'-');
    if negative || bytes.first() == Some(&b'+') {
        i += 1;
    }

    let mut integer: i32 = 0;
    let integer_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        integer = integer
            .saturating_mul(10)
            .saturating_add(i32::from(bytes[i] - b'0'));
        i += 1;
    }
    let mut fraction = 0;
    let mut has_fraction = false;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        let mut scale = 10;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            fraction += i32::from(bytes[i] - b'0') * scale;
            scale /= 10;
            has_fraction = true;
            i += 1;
        }
    }
    if i == integer_start
        || (i == integer_start + 1 && !has_fraction && bytes[integer_start] == b'.')
    {
        return None;
    }

    let number = integer.saturating_mul(100).saturating_add(fraction);
    Some((if negative { -number } else { number }, &value[i..]))
}

fn components(value: &str) -> impl Iterator<Item = &str> {
    let mut rest = value;
    core::iter::from_fn(move || {
        loop {
            let first = rest.chars().next()?;
            let end = component_end(rest);
            if end == 0 {
                rest = &rest[first.len_utf8()..];
                continue;
            }
            let component = &rest[..end];
            rest = &rest[end..];
            return Some(component);
        }
    })
}

fn component_end(text: &str) -> usize {
    let mut depth = 0;
    let mut quote = 0;

    for (index, &byte) in text.as_bytes().iter().enumerate() {
        if quote != 0 {
            if byte == quote {
                quote = 0;
            }
            continue;
        }
        match byte {
            b'"' | b'\'' => quote = byte,
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' | b'/' if depth == 0 => return index,
            _ if depth == 0 && byte.is_ascii_whitespace() => return index,
            _ => {}
        }
    }

    text.len()
}

fn split_function(value: &str) -> Option<(&str, &str)> {
    let open = value.find('(')?;
    let name = &value[..open];
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    let close = open + 1 + matching_paren(&value[open + 1..])?;
    (close == value.len() - 1).then(|| (name, &value[open + 1..close]))
}

fn matching_paren(text: &str) -> Option<usize> {
    let mut depth = 0;
    for (index, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(index),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

pub fn split_top_level(text: &str, separator: u8) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut quote = 0;
    let mut start = 0;

    for (index, &byte) in text.as_bytes().iter().enumerate() {
        if quote != 0 {
            if byte == quote {
                quote = 0;
            }
            continue;
        }
        match byte {
            b'"' | b'\'' => quote = byte,
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            _ if depth == 0 && byte == separator => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());

    parts
}
