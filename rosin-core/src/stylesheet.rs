#![forbid(unsafe_code)]

use crate::parser::*;
use crate::prelude::Key;
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
        self.specificity.cmp(&other.specificity)
    }
}

impl PartialOrd for Rule {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Default, Clone)]
struct StylesheetInner {
    dynamic_rules: Vec<Rule>,
    static_rules: Vec<Rule>,
}

#[derive(Default, Clone)]
pub struct Stylesheet {
    inner: Arc<RwLock<StylesheetInner>>,
}

impl std::fmt::Debug for Stylesheet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stylesheet").finish()
    }
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
pub(crate) fn apply_static_styles<S, H>(temp: &Bump, tree: &[ArrayNode<S, H>], styles: &mut BumpVec<'static, Style>) {
    let mut sheets = BumpVec::new_in(temp);
    let mut parent_id = usize::MAX;

    for id in 0..tree.len() {
        styles.push(Style::default());

        // Re-use ancestor sheets from siblings to reduce walks up the tree
        if parent_id != tree[id].parent {
            sheets.clear();
            let mut ancestor = tree[id].parent;
            while ancestor != usize::MAX {
                if let Some(stylesheet) = &tree[ancestor].style_sheet {
                    sheets.push(stylesheet);
                }
                ancestor = tree[ancestor].parent;
            }
            parent_id = tree[id].parent;
        }

        // Find matching rules
        let rule_filter = |rule: &&Rule| {
            let mut direct = false;
            let mut cmp_node = id;
            let mut first = true;
            for selector in rule.selectors.iter().rev() {
                while cmp_node != usize::MAX {
                    match selector {
                        Selector::Wildcard => {
                            cmp_node = tree[cmp_node].parent;
                            direct = false;
                            break; // Next selector
                        }
                        Selector::Id(_) | Selector::Class(_) => {
                            if selector.check(&tree[cmp_node]) {
                                cmp_node = tree[cmp_node].parent;
                                direct = false;
                                break; // Next selector
                            } else if direct || first {
                                return false; // Must match, but didn't
                            }

                            cmp_node = tree[cmp_node].parent;
                            direct = false;
                            first = false;
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
            }
            true // All selectors satisfied
        };

        let parent_style: Option<Style> = if id == 0 { None } else { Some(styles[tree[id].parent].clone()) };

        // Find the font size, family, and color (Used for relative lengths and currentColor)
        let mut font_size_set = false;
        let mut font_family_set = false;
        let mut color_set = false;

        tree[id].style_sheet.as_ref().iter().chain(sheets.iter()).rev().for_each(|sheet| {
            if font_size_set && font_family_set && color_set {
                return;
            }

            let rule_action = |rule: &Rule| {
                for property in rule.properties.iter().rev() {
                    if font_size_set && font_family_set && color_set {
                        return;
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
            };

            sheet
                .inner
                .read()
                .unwrap()
                .static_rules
                .iter()
                .filter(rule_filter)
                .for_each(rule_action);
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

        tree[id].style_sheet.as_ref().iter().chain(sheets.iter()).for_each(|sheet| {
            sheet
                .inner
                .read()
                .unwrap()
                .static_rules
                .iter()
                .filter(rule_filter)
                .for_each(|rule| {
                    for property in &rule.properties {
                        match property {
                            Property::FontSize(_) | Property::Color(_) | Property::FontFamily(_) => continue,
                            _ => {}
                        }
                        property.apply(&mut styles[id], &parent_style);
                    }
                });
        });
    }
}

// Perform selector matching and apply dynamic styles to a tree
pub(crate) fn apply_dynamic_styles<S, H>(
    temp: &Bump,
    tree: &[ArrayNode<S, H>],
    focused_node: Option<Key>,
    hot_nodes: &[usize],
    styles: &mut [Style],
    default_styles: &mut BumpVec<'_, (usize, Style)>,
) {
    let mut sheets = BumpVec::new_in(temp);
    let mut parent_id = usize::MAX;

    for id in 0..tree.len() {
        // Re-use ancestor sheets from siblings to reduce walks up the tree
        if parent_id != tree[id].parent {
            sheets.clear();
            let mut ancestor = tree[id].parent;
            while ancestor != usize::MAX {
                if let Some(stylesheet) = &tree[ancestor].style_sheet {
                    sheets.push(stylesheet);
                }
                ancestor = tree[ancestor].parent;
            }
            parent_id = tree[id].parent;
        }

        // Find matching rules
        let rule_filter = |rule: &&Rule| {
            let mut direct = false;
            let mut cmp_node = id;
            let mut first = true;
            for selector in rule.selectors.iter().rev() {
                while cmp_node != usize::MAX {
                    match selector {
                        Selector::Wildcard => {
                            cmp_node = tree[cmp_node].parent;
                            direct = false;
                            break; // Next selector
                        }
                        Selector::Id(_) | Selector::Class(_) => {
                            if selector.check(&tree[cmp_node]) {
                                cmp_node = tree[cmp_node].parent;
                                direct = false;
                                break; // Next selector
                            } else if direct || first {
                                return false; // Must match, but didn't
                            }

                            cmp_node = tree[cmp_node].parent;
                            direct = false;
                            first = false;
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
                        Selector::Hover => {
                            if hot_nodes.contains(&cmp_node) {
                                break; // Next selector
                            } else {
                                return false;
                            }
                        }
                        Selector::Focus => {
                            if let (Some(cmp_key), Some(focus_key)) = (tree[cmp_node].key, focused_node) {
                                if cmp_key == focus_key {
                                    break; // Next selector
                                } else {
                                    return false;
                                }
                            } else {
                                return false;
                            }
                        }
                    }
                }
            }
            true // All selectors satisfied
        };

        let parent_style: Option<Style> = if id == 0 { None } else { Some(styles[tree[id].parent].clone()) };

        tree[id].style_sheet.as_ref().iter().chain(sheets.iter()).for_each(|sheet| {
            sheet
                .inner
                .read()
                .unwrap()
                .dynamic_rules
                .iter()
                .filter(rule_filter)
                .for_each(|rule| {
                    if let Some((default_id, _)) = &default_styles.last() {
                        if *default_id != id {
                            default_styles.push((id, styles[id].clone()));
                        }
                    } else {
                        default_styles.push((id, styles[id].clone()));
                    }

                    for property in &rule.properties {
                        property.apply(&mut styles[id], &parent_style);
                    }
                });
        });
    }
}
