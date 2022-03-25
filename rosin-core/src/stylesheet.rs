#![forbid(unsafe_code)]

use crate::parser::*;
use crate::properties::*;
use crate::resource::ParseResource;
use crate::resource::ResourceLoader;
use crate::style::*;
use crate::tree::*;

use bumpalo::collections::CollectIn;
use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump;
use cssparser::{Parser, ParserInput, RuleListParser};

use std::cmp::Ordering;
use std::sync::Arc;
use std::sync::Mutex;

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

    // Represents a `:hover` selector
    Hover,

    // Represents a `:focus` selector
    Focus,
}

impl Selector {
    // Check if this selector applies to a node
    pub(crate) fn check<S, H>(&self, node: &ArrayNode<S, H>) -> bool {
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
    pub rules: Vec<Rule>,
    pub dynamic_rules: Vec<Rule>,
}

impl ParseResource for Stylesheet {
    // Parse CSS text into rule list
    fn parse(text: &str) -> Self {
        let mut input = ParserInput::new(text);
        let mut parser = Parser::new(&mut input);
        let mut rules = Vec::new();
        let mut dynamic_rules = Vec::new();

        for (dynamic, rule) in RuleListParser::new_for_stylesheet(&mut parser, RulesParser).flatten() {
            if dynamic {
                dynamic_rules.push(rule);
            } else {
                rules.push(rule);
            }
        }

        Self { rules, dynamic_rules }
    }
}

// Perform selector matching and apply styles to a tree, ignoring hover/focus
pub(crate) fn apply_styles<S, H>(temp: &Bump, tree: &mut [ArrayNode<S, H>], rl: Arc<Mutex<ResourceLoader>>) {
    for id in 0..tree.len() {
        // TODO - benchmark hash map
        let rl = rl.lock().unwrap();
        let mut relevant_rules = rl
            .get_sheet(tree[0].style_sheet.unwrap())
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
                                    Selector::Hover | Selector::Focus => {
                                        // Hover and Focus styles aren't applied in this step
                                        return false;
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
            .collect_in::<BumpVec<&Rule>>(temp);

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
