use alloc::string::String;
use alloc::vec::Vec;

pub struct StyleContext {
    viewport_width: i32,
    custom_properties: Vec<(String, String)>,
    centered_selectors: Vec<String>,
}

impl StyleContext {
    pub const fn new(viewport_width: i32) -> Self {
        StyleContext {
            viewport_width,
            custom_properties: Vec::new(),
            centered_selectors: Vec::new(),
        }
    }

    pub fn viewport_width(&self) -> i32 {
        self.viewport_width
    }

    pub fn set_custom_property(&mut self, name: &str, value: &str) {
        match self
            .custom_properties
            .iter_mut()
            .find(|(existing, _)| existing == name)
        {
            Some((_, existing_value)) => *existing_value = String::from(value),
            None => self
                .custom_properties
                .push((String::from(name), String::from(value))),
        }
    }

    pub fn custom_property(&self, name: &str) -> Option<&str> {
        self.custom_properties
            .iter()
            .find(|(existing, _)| existing == name)
            .map(|(_, value)| value.as_str())
    }

    pub fn is_centered(&self, selectors: &str) -> bool {
        self.centered_selectors
            .iter()
            .any(|centered| centered == selectors)
    }

    pub fn set_centered(&mut self, selectors: &str, centered: bool) {
        self.centered_selectors
            .retain(|centered| centered != selectors);
        if centered {
            self.centered_selectors.push(String::from(selectors));
        }
    }
}
