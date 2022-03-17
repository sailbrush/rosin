#![forbid(unsafe_code)]

use crate::parser::*;
use crate::properties::*;
use crate::style::*;
use crate::tree::*;

use cssparser::{Parser, ParserInput, RuleListParser};

use std::{cmp::Ordering, error::Error, fs, time::SystemTime};

#[derive(Debug, Clone)]
pub enum Selector {
    // Represents a `*` selector
    Wildcard,

    // Represents selectors beginning with `#`
    Id(String),

    // Represents selectors beginning with `.`
    Class(String),

    // Represents a ` ` selector relationship
    Children,

    // Represents a `>` selector relationship
    DirectChildren,
    // TODO - Represents a `:hover` selector
    //Hover,

    // TODO - Represents a `:focus` selector
    //Focus,
}

impl Selector {
    // Check if this selector applies to a node
    pub(crate) fn check<T>(&self, node: &ArrayNode<T>) -> bool {
        match self {
            Selector::Wildcard => true,
            Selector::Id(selector) | Selector::Class(selector) => node.classes.iter().any(|class| class == selector),
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub specificity: u32,
    pub selectors: Vec<Selector>,
    pub properties: Vec<Property>,
}

impl Eq for Rule {}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        self.specificity == other.specificity
    }
}

impl Ord for Rule {
    fn cmp(&self, other: &Self) -> Ordering {
        self.specificity.cmp(&other.specificity)
    }
}

impl PartialOrd for Rule {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct Stylesheet {
    pub path: Option<&'static str>,
    pub last_modified: Option<SystemTime>,
    pub rules: Vec<Rule>,
}

#[doc(hidden)]
impl Stylesheet {
    pub fn new_static(text: &'static str) -> Self {
        Self {
            path: None,
            last_modified: None,
            rules: Self::parse(text),
        }
    }

    pub fn new_dynamic(path: &'static str) -> Self {
        let mut new = Self {
            path: Some(path),
            last_modified: None,
            rules: Vec::new(),
        };
        new.poll().expect("[Rosin] Failed to load stylesheet.");
        new
    }

    // Parse CSS text into rule list
    pub fn parse(text: &str) -> Vec<Rule> {
        let mut input = ParserInput::new(text);
        let mut parser = Parser::new(&mut input);
        let mut rules_list = Vec::new();

        for rule in RuleListParser::new_for_stylesheet(&mut parser, RulesParser).flatten() {
            rules_list.push(rule);
        }
        rules_list
    }

    // Reload stylesheet if it changed on disk
    pub(crate) fn poll(&mut self) -> Result<bool, Box<dyn Error>> {
        if let Some(path) = self.path {
            let mut reload = true;
            let last_modified = fs::metadata(&path)?.modified()?;

            if let Some(prev_last_modified) = self.last_modified {
                if last_modified == prev_last_modified {
                    reload = false;
                }
            }

            if reload {
                self.last_modified = Some(last_modified);
                let contents = fs::read_to_string(path)?;
                self.rules = Self::parse(&contents);
            }

            Ok(reload)
        } else {
            Ok(false)
        }
    }

    // Perform selector matching and apply styles to a tree
    pub(crate) fn apply_style<T>(&self, tree: &mut [ArrayNode<T>]) {
        for id in 0..tree.len() {
            // TODO - benchmark hash map
            // TODO - use temp bump alloc instead of Vec
            let mut relevant_rules = self
                .rules
                .iter()
                .filter(|rule| {
                    // Find matching rules
                    // TODO - comment an explanation
                    let mut direct = false;
                    let mut cmp_node = Some(id);
                    for (i, selector) in rule.selectors.iter().rev().enumerate() {
                        loop {
                            if let Some(n) = cmp_node {
                                if i == 0 {
                                    if !selector.check(&tree[n]) {
                                        return false;
                                    } else {
                                        cmp_node = if n != 0 { Some(tree[n].parent) } else { None };
                                        break; // Next selector
                                    }
                                } else {
                                    match selector {
                                        Selector::Wildcard => {
                                            cmp_node = if n != 0 { Some(tree[n].parent) } else { None };
                                            direct = false;
                                            break; // Next selector
                                        }
                                        Selector::Id(_) | Selector::Class(_) => {
                                            cmp_node = if n != 0 { Some(tree[n].parent) } else { None };

                                            if selector.check(&tree[n]) {
                                                direct = false;
                                                break; // Next selector
                                            } else if direct {
                                                return false; // Must match, but didn't
                                            }

                                            direct = false;
                                            continue; // Don't go to the next selector, just move up the tree
                                        }
                                        Selector::DirectChildren => {
                                            direct = true;
                                            break; // Next selector
                                        }
                                        Selector::Children => {
                                            direct = false;
                                            break; // Next selector
                                        }
                                    }
                                }
                            } else {
                                return false; // Made it to the root unsasitfied
                            }
                        }
                    }
                    true // All selectors satisfied
                })
                .collect::<Vec<&Rule>>();

            let par_style: Option<Style> = if id == 0 { None } else { Some(tree[tree[id].parent].style.clone()) };

            relevant_rules.sort();

            // First find the font size and color (Used for relative lengths and currentColor)
            let mut font_size_set = false;
            let mut font_family_set = false;
            let mut color_set = false;
            relevant_rules.iter().for_each(|rule| {
                if font_size_set && font_family_set && color_set {
                    return;
                }
                for property in rule.properties.iter().rev() {
                    if font_size_set && font_family_set && color_set {
                        break;
                    }
                    match property {
                        Property::FontSize(value) => {
                            if font_size_set {
                                continue;
                            }
                            match value {
                                PropertyValue::Inherit => {
                                    if let Some(parent) = &par_style {
                                        tree[id].style.font_size = parent.font_size;
                                    }
                                }
                                PropertyValue::Exact(size) => match size {
                                    Length::Px(value) => {
                                        tree[id].style.font_size = *value;
                                    }
                                    Length::Em(value) => {
                                        if let Some(parent) = &par_style {
                                            tree[id].style.font_size = parent.font_size * value;
                                        } else {
                                            tree[id].style.font_size *= value;
                                        }
                                    }
                                },
                                _ => {}
                            };
                            font_size_set = true;
                        }
                        Property::FontFamily(value) => {
                            if font_family_set {
                                continue;
                            }
                            match value {
                                PropertyValue::Exact(family) => {
                                    tree[id].style.font_family = Some(family.clone());
                                }
                                _ => {
                                    // Inherited by default
                                    if let Some(parent) = &par_style {
                                        tree[id].style.font_family = parent.font_family.clone();
                                    }
                                }
                            }
                            font_family_set = true;
                        }
                        Property::Color(value) => {
                            if color_set {
                                continue;
                            }
                            match value {
                                PropertyValue::Initial => tree[id].style.color = Style::default().color,
                                PropertyValue::Exact(color) => {
                                    if let cssparser::Color::RGBA(rgba) = color {
                                        tree[id].style.color = *rgba;
                                    }
                                }
                                _ => {
                                    // Inherited by default
                                    if let Some(parent) = &par_style {
                                        tree[id].style.color = parent.color;
                                    }
                                }
                            }
                            color_set = true;
                        }
                        _ => {}
                    }
                }
            });
            if !font_size_set {
                if let Some(parent) = &par_style {
                    tree[id].style.font_size = parent.font_size;
                }
            }
            if !font_family_set {
                if let Some(parent) = &par_style {
                    tree[id].style.font_family = parent.font_family.clone();
                }
            }
            if !color_set {
                if let Some(parent) = &par_style {
                    tree[id].style.color = parent.color;
                }
            }

            relevant_rules.iter().for_each(|rule| {
                apply_properties(&rule.properties, &mut tree[id].style, &par_style);
            });
        }
    }
}
