use crate::html;
use crate::value::split_top_level;

use alloc::string::String;
use alloc::vec::Vec;

const SUPPORTED_PSEUDO_CLASSES: &[&str] = &[
    "link",
    "visited",
    "hover",
    "active",
    "first-line",
    "first-letter",
];

pub fn translate_selector_list(list: &str) -> Vec<String> {
    split_top_level(list, b',')
        .into_iter()
        .filter_map(translate_selector)
        .collect()
}

pub fn selects_root(list: &str) -> bool {
    split_top_level(list, b',')
        .into_iter()
        .any(|selector| matches!(selector, ":root" | "html"))
}

fn translate_selector(selector: &str) -> Option<String> {
    if selector.contains(['+', '~', '[']) {
        return None;
    }

    let compounds: Option<Vec<String>> = selector
        .split(|c: char| c == '>' || c.is_ascii_whitespace())
        .filter(|compound| !compound.is_empty())
        .map(translate_compound)
        .collect();
    let compounds = compounds?;
    (!compounds.is_empty()).then(|| compounds.join(" "))
}

fn translate_compound(compound: &str) -> Option<String> {
    let type_end = compound.find(['.', '#', ':']).unwrap_or(compound.len());
    let (element, rest) = compound.split_at(type_end);
    let mut translated = translate_element(element, rest.is_empty());

    for (index, part) in rest.match_indices(['.', '#', ':']) {
        let end = rest[index + 1..]
            .find(['.', '#', ':'])
            .map_or(rest.len(), |end| index + 1 + end);
        let name = &rest[index + 1..end];
        match part {
            ":" if name.is_empty() => return None,
            ":" if name == "root" => translated = String::from("html"),
            ":" if SUPPORTED_PSEUDO_CLASSES.contains(&name) => {
                translated.push(':');
                translated.push_str(name);
            }
            ":" => return None,
            _ => {
                translated.push_str(part);
                translated.push_str(name);
            }
        }
    }

    Some(translated)
}

fn translate_element(element: &str, alone: bool) -> String {
    let element = element.to_ascii_lowercase();
    if html::is_renamed_element(element.as_bytes()) {
        let mut translated = String::from("div.");
        translated.push_str(&html::renamed_element_class(&element));
        return translated;
    }
    if element == "*" && !alone {
        return String::new();
    }
    element
}
