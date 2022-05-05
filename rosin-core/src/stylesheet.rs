#![forbid(unsafe_code)]

use crate::parser::*;
use crate::properties::*;
use crate::style::*;
use crate::tree::*;

use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump;
use cssparser::{Parser, ParserInput, RuleListParser};

use std::cmp::Ordering;
use std::sync::Arc;
use std::sync::RwLock;

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
        other.specificity.cmp(&self.specificity)
    }
}

impl PartialOrd for Rule {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.cmp(self))
    }
}

#[derive(Debug, Default, Clone)]
struct StylesheetInner {
    dynamic_rules: Vec<Rule>,
    static_rules: Vec<Rule>,
}

#[derive(Debug, Default, Clone)]
pub struct Stylesheet {
    inner: Arc<RwLock<StylesheetInner>>,
}

impl Stylesheet {
    // Parse CSS text into rule list
    pub(crate) fn parse(text: &str) -> Self {
        let mut input = ParserInput::new(text);
        let mut parser = Parser::new(&mut input);
        let mut dynamic_rules = Vec::new();
        let mut static_rules = Vec::new();

        for (dynamic, rule) in RuleListParser::new_for_stylesheet(&mut parser, RulesParser).flatten() {
            if dynamic {
                dynamic_rules.push(rule);
            } else {
                static_rules.push(rule);
            }
        }

        dynamic_rules.sort();
        static_rules.sort();

        Self {
            inner: Arc::new(RwLock::new(StylesheetInner {
                dynamic_rules,
                static_rules,
            })),
        }
    }

    pub(crate) fn reparse(&mut self, text: &str) {
        if let Ok(mut data) = self.inner.try_write() {
            let mut input = ParserInput::new(text);
            let mut parser = Parser::new(&mut input);

            data.dynamic_rules.clear();
            data.static_rules.clear();

            for (dynamic, rule) in RuleListParser::new_for_stylesheet(&mut parser, RulesParser).flatten() {
                if dynamic {
                    data.dynamic_rules.push(rule);
                } else {
                    data.static_rules.push(rule);
                }
            }

            data.dynamic_rules.sort();
            data.static_rules.sort();
        }
    }
}

// Perform selector matching and apply styles to a tree
pub(crate) fn apply_styles<S, H>(dynamic: bool, temp: &Bump, tree: &[ArrayNode<S, H>], styles: &mut BumpVec<'static, Style>) {
    for id in 0..tree.len() {
        styles.push(Style::default());

        let rule_filter = |rule: &&Rule| {
            // Find matching rules
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
        };

        // TODO: should be able to reduce the number of walks up the tree by re-using data from siblings
        let mut sheets = BumpVec::new_in(temp);
        let mut ancestor = id;
        let mut saw_root = false;
        while ancestor != 0 || !saw_root {
            if ancestor == 0 {
                saw_root = true;
            }

            if let Some(stylesheet) = &tree[ancestor].style_sheet {
                sheets.push(stylesheet);
            }

            ancestor = tree[ancestor].parent;
        }

        let parent_style: Option<Style> = if id == 0 { None } else { Some(styles[tree[id].parent].clone()) };

        // First find the font size and color (Used for relative lengths and currentColor)
        let mut font_size_set = false;
        let mut font_family_set = false;
        let mut color_set = false;
        sheets.iter().for_each(|sheet| {
            let guard = sheet.inner.read().unwrap();
            let list = if dynamic {
                &guard.dynamic_rules
            } else {
                &guard.static_rules
            };

            list
                .iter()
                .filter(rule_filter)
                .for_each(|rule| {
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
                                        if let Some(parent) = &parent_style {
                                            styles[id].font_size = parent.font_size;
                                        }
                                    }
                                    PropertyValue::Exact(size) => match size {
                                        Length::Px(value) => {
                                            styles[id].font_size = *value;
                                        }
                                        Length::Em(value) => {
                                            if let Some(parent) = &parent_style {
                                                styles[id].font_size = parent.font_size * value;
                                            } else {
                                                styles[id].font_size *= value;
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
                                        styles[id].font_family = Some(family.clone());
                                    }
                                    _ => {
                                        // Inherited by default
                                        if let Some(parent) = &parent_style {
                                            styles[id].font_family = parent.font_family.clone();
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
                                    PropertyValue::Initial => styles[id].color = Style::default().color,
                                    PropertyValue::Exact(color) => {
                                        if let cssparser::Color::RGBA(rgba) = color {
                                            styles[id].color = *rgba;
                                        }
                                    }
                                    _ => {
                                        // Inherited by default
                                        if let Some(parent) = &parent_style {
                                            styles[id].color = parent.color;
                                        }
                                    }
                                }
                                color_set = true;
                            }
                            _ => {}
                        }
                    }
                })
        });

        if !font_size_set {
            if let Some(parent) = &parent_style {
                styles[id].font_size = parent.font_size;
            }
        }
        if !font_family_set {
            if let Some(parent) = &parent_style {
                styles[id].font_family = parent.font_family.clone();
            }
        }
        if !color_set {
            if let Some(parent) = &parent_style {
                styles[id].color = parent.color;
            }
        }

        // TODO: Should be able to run the filter over the rules only once
        sheets.iter().for_each(|sheet| {
            let guard = sheet.inner.read().unwrap();
            let list = if dynamic {
                &guard.dynamic_rules
            } else {
                &guard.static_rules
            };

            list
                .iter()
                .filter(rule_filter)
                .for_each(|rule| {
                    apply_properties(&rule.properties, &mut styles[id], &parent_style);
                });
        });
    }
}
