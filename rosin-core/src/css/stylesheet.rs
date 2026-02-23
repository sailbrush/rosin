use std::{collections::HashMap, fmt::Display, fs, path::Path, str::FromStr, sync::Arc};

use bumpalo::{Bump, collections::Vec as BumpVec};
use cssparser::{Parser, ParserInput, RuleBodyParser};
use log::error;
use parking_lot::{RwLock, RwLockReadGuard};
use qfilter::Filter;
use smallvec::SmallVec;

use crate::{
    css::{self, parser::*, properties::*, style::*},
    interner::StrId,
    prelude::*,
    tree::Node,
    util::ResourceInfo,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Selector {
    /// Represents a `.class` selector (interned)
    Class(StrId),

    /// Represents a `*` selector
    Wildcard,

    /// Represents a "space" (U+0020) combinator
    Descendant,

    /// Represents a `>` combinator
    Child,

    /// Represents a `:hover` pseudo-class
    Hover,

    /// Represents a `:focus` pseudo-class
    Focus,

    /// Represents a `:active` pseudo-class
    Active,

    /// Represents a `:disabled` pseudo-class
    Disabled,

    /// Represents a `:enabled` pseudo-class
    Enabled,
}

impl Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Selector::Class(value) => write!(f, ".{}", value),
            Selector::Wildcard => f.write_str("*"),
            Selector::Descendant => f.write_str(" "),
            Selector::Child => f.write_str(" > "),
            Selector::Hover => f.write_str(":hover"),
            Selector::Focus => f.write_str(":focus"),
            Selector::Active => f.write_str(":active"),
            Selector::Disabled => f.write_str(":disabled"),
            Selector::Enabled => f.write_str(":enabled"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Rule {
    pub selectors: Vec<Selector>,
    pub properties: Arc<SmallVec<[Property; 2]>>,
    pub specificity: u32,
    pub has_pseudos: bool,
    pub variables: Vec<(Arc<str>, Arc<str>)>,
}

impl Eq for Rule {}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        self.specificity == other.specificity
            && self.selectors == other.selectors
            && self.properties.as_ref() == other.properties.as_ref()
            && self.variables == other.variables
    }
}

impl Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for selector in &self.selectors {
            write!(f, "{selector}")?;
        }

        f.write_str(" {")?;

        for property in self.properties.iter() {
            write!(f, "\t{property}")?;
        }

        f.write_str("}\n")
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct StylesheetInner {
    pub info: Option<ResourceInfo>,
    pub rules: Vec<Rule>,

    /// Inverted index for `*`
    pub wildcard: Vec<usize>,

    /// Inverted index for `.class`
    pub index: HashMap<StrId, Vec<usize>>,
}

/// A parsed CSS stylesheet.
#[derive(Default, Clone)]
pub struct Stylesheet {
    pub(crate) inner: Arc<RwLock<StylesheetInner>>,
}

impl Eq for Stylesheet {}
impl PartialEq for Stylesheet {
    fn eq(&self, other: &Self) -> bool {
        let self_inner = self.inner.read();
        let other_inner = other.inner.read();

        self_inner.rules == other_inner.rules
    }
}

impl std::fmt::Debug for Stylesheet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stylesheet").finish_non_exhaustive()
    }
}

impl Display for Stylesheet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.read();

        for rule in &inner.rules {
            writeln!(f, "{rule}")?;
        }

        Ok(())
    }
}

impl FromStr for Stylesheet {
    type Err = ();

    fn from_str(css: &str) -> Result<Self, Self::Err> {
        let mut rules = Vec::new();
        let mut wildcard = Vec::new();
        let mut index = HashMap::new();
        Self::parse(css, None, &mut rules, &mut wildcard, &mut index);

        Ok(Self {
            inner: Arc::new(RwLock::new(StylesheetInner {
                info: None,
                rules,
                wildcard,
                index,
            })),
        })
    }
}

impl Stylesheet {
    /// Parses a CSS file. In debug builds, the file will be reloaded when changed on disk.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Stylesheet, std::io::Error> {
        let path_buf = path.as_ref().canonicalize()?;
        let css = fs::read_to_string(&path_buf)?;
        let mut rules = Vec::new();
        let mut wildcard = Vec::new();
        let mut index = HashMap::new();
        Self::parse(&css, Some(&path_buf), &mut rules, &mut wildcard, &mut index);

        let info = Some(ResourceInfo {
            last_modified: fs::metadata(&path_buf)?.modified()?,
            path: path_buf.clone(),
        });

        Ok(Self {
            inner: Arc::new(RwLock::new(StylesheetInner { info, rules, wildcard, index })),
        })
    }

    pub(crate) fn reload(&mut self) -> Result<bool, std::io::Error> {
        let mut write_guard = self.inner.write();
        let inner = &mut *write_guard;

        if let Some(ref mut resource_info) = inner.info {
            let current_modified_time = fs::metadata(&resource_info.path)?.modified()?;

            if current_modified_time > resource_info.last_modified {
                // File has been modified, reload it
                let new_css = fs::read_to_string(&resource_info.path)?;
                Self::parse(&new_css, Some(&resource_info.path), &mut inner.rules, &mut inner.wildcard, &mut inner.index);

                resource_info.last_modified = current_modified_time;

                return Ok(true);
            }
        }

        Ok(false)
    }

    fn parse(css: &str, file_name: Option<&Path>, rules: &mut Vec<Rule>, wildcard: &mut Vec<usize>, index: &mut HashMap<StrId, Vec<usize>>) {
        rules.clear();
        wildcard.clear();
        index.clear();

        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let mut rp = RulesParser { file_name };

        for result in RuleBodyParser::new(&mut parser, &mut rp) {
            match result {
                Ok(parsed_rules) => {
                    for rule in parsed_rules {
                        rules.push(rule);
                    }
                }
                Err((error, css)) => {
                    let msg = format_args!("Failed to parse CSS rule: `{}`", css.lines().next().unwrap_or(""));
                    css::log_error(msg, error.location, file_name);
                }
            }
        }

        // Need to use a stable sort because order in the stylesheet is also important
        rules.sort_by_key(|r| r.specificity);

        // Build indexes
        for (idx, rule) in rules.iter().enumerate() {
            let mut indexed = false;

            for selector in rule.selectors.iter().rev() {
                match selector {
                    // Skip pseudos when choosing an index key
                    Selector::Hover | Selector::Focus | Selector::Active | Selector::Disabled | Selector::Enabled => {
                        continue;
                    }

                    // If we reach a combinator before finding a concrete key,
                    // the rightmost simple selector is effectively "any element"
                    // (`.a > :hover`), so treat it as a wildcard.
                    Selector::Wildcard | Selector::Child | Selector::Descendant => {
                        wildcard.push(idx);
                        indexed = true;
                        break;
                    }
                    Selector::Class(class_id) => {
                        index.entry(*class_id).or_default().push(idx);
                        indexed = true;
                        break;
                    }
                }
            }

            // If the selector list had no concrete key, index it as wildcard.
            if !indexed {
                wildcard.push(idx);
            }
        }
    }
}

#[derive(Copy, Clone)]
enum PseudoKind {
    Hover,
    Focus,
    Active,
    Enabled, // covers :enabled and :disabled
}

#[derive(Debug)]
pub(crate) struct VariableContext {
    /// Maps a variable name to a stack of values. The last value is the current one.
    css_vars: HashMap<Arc<str>, Vec<Arc<str>>>,
    /// Tracks which variables were added at which node index to allow efficient popping.
    scope_history: Vec<(usize, Vec<Arc<str>>)>,
}

impl VariableContext {
    fn new() -> Self {
        Self {
            css_vars: HashMap::with_capacity(64),
            scope_history: Vec::with_capacity(32),
        }
    }

    /// Empties the context.
    fn clear(&mut self) {
        self.css_vars.clear();
        self.scope_history.clear();
    }

    /// Get the current value of a variable.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.css_vars.get(name).and_then(|stack| stack.last()).map(|v| &**v)
    }

    /// Add a set of variables to the node's scope.
    fn push_vars(&mut self, node_idx: usize, vars: &[(Arc<str>, Arc<str>)]) {
        if vars.is_empty() {
            return;
        }

        let mut keys_added = Vec::with_capacity(vars.len());

        for (name, value) in vars {
            self.css_vars.entry(Arc::clone(name)).or_default().push(value.clone());
            keys_added.push(name.clone());
        }

        self.scope_history.push((node_idx, keys_added));
    }

    /// Remove the most recent set of variables added to the context.
    fn pop_vars(&mut self) {
        if let Some((_, keys)) = self.scope_history.pop() {
            for key in keys {
                if let Some(stack) = self.css_vars.get_mut(&key) {
                    stack.pop();
                    if stack.is_empty() {
                        self.css_vars.remove(&key);
                    }
                }
            }
        }
    }

    /// Remove scopes that are no longer ancestors of the current node.
    fn prune(&mut self, current_parent_idx: usize) {
        while let Some((scope_node_idx, _)) = self.scope_history.last() {
            if *scope_node_idx > current_parent_idx {
                self.pop_vars();
            } else {
                break;
            }
        }
    }
}

/// Builds per-node CSS variable cache from static rules and computes `style_flags` for each node.
pub(crate) fn style_pre_pass<S, H>(temp: &Bump, tree: &mut Ui<S, H>, ancestor_classes: &mut Filter) {
    debug_assert_eq!(tree.var_scope_cache.len(), 0, "Expected empty var_scope_cache before pre-pass.");
    debug_assert_eq!(tree.style_flags.len(), 0, "Expected empty style_flags before pre-pass.");

    ancestor_classes.clear();

    let mut active_sheets: BumpVec<(usize, Stylesheet)> = BumpVec::new_in(temp);
    let mut rules_list: BumpVec<usize> = BumpVec::new_in(temp);
    let mut var_delta: HashMap<Arc<str>, Arc<str>> = HashMap::with_capacity(16);
    let mut pseudos: Vec<(PseudoKind, usize)> = Vec::with_capacity(32);

    for idx in 0..tree.nodes.len() {
        tree.style_flags.push(0);

        update_sheets(&tree.nodes, idx, &mut active_sheets);
        update_ancestors(&tree.nodes, idx, ancestor_classes);

        var_delta.clear();

        // Walk active stylesheets from nearest to farthest ancestor
        for (_, (_, sheet)) in active_sheets.iter().enumerate().rev() {
            rules_list.clear();

            let inner = sheet.inner.read();

            rules_list.extend_from_slice(&inner.wildcard);
            for &class_id in tree.nodes[idx].classes.iter() {
                if let Some(indexes) = inner.index.get(&class_id) {
                    rules_list.extend_from_slice(indexes);
                }
            }

            rules_list.sort_unstable();
            rules_list.dedup();

            for &rule_idx in rules_list.iter() {
                let rule = &inner.rules[rule_idx];

                if !rule.has_pseudos {
                    // Static rule, build var delta
                    if rule_matches_node(rule, &tree.nodes, idx, None, None, &[], ancestor_classes, None) {
                        for (name, value) in &rule.variables {
                            // later overwrites earlier
                            var_delta.insert(Arc::clone(name), Arc::clone(value));
                        }
                    }
                } else {
                    // Dynamic rule, compute invalidation flags
                    let start = pseudos.len();

                    let matched = rule_matches_node(rule, &tree.nodes, idx, None, None, &[], ancestor_classes, Some(&mut pseudos));
                    if !matched {
                        pseudos.truncate(start);
                        continue;
                    }

                    // Apply flags for pseudos added by this rule, then clear them.
                    for &(kind, pseudo_node_idx) in &pseudos[start..] {
                        let bit = match kind {
                            PseudoKind::Hover => css::HOVER_DIRTY,
                            PseudoKind::Focus => css::FOCUS_DIRTY,
                            PseudoKind::Active => css::ACTIVE_DIRTY,
                            PseudoKind::Enabled => css::ENABLED_DIRTY,
                        };

                        tree.style_flags[pseudo_node_idx] |= bit;
                    }

                    pseudos.truncate(start);
                }
            }
        }

        tree.var_scope_cache
            .push(var_delta.iter().map(|(k, v)| (Arc::clone(k), Arc::clone(v))).collect());
    }
}

/// Perform selector matching and apply CSS classes to a tree.
///
/// Returns true if one of the rules could have affected layout.
pub(crate) fn style_pass<S, H>(
    temp: &Bump,
    tree: &mut Ui<S, H>,
    state: &S,
    focused_node: Option<NodeId>,
    active_node: Option<NodeId>,
    hot_nodes: &[usize],
    ancestor_classes: &mut Filter,
) -> bool {
    tree.merge_dirty_roots();
    if tree.nodes.is_empty() || tree.dirty_roots.is_empty() {
        return false;
    }

    // Split tree into disjoint borrows
    let Ui {
        nodes,
        style_cache,
        dirty_roots,
        var_scope_cache,
        on_style_deps,
        ..
    } = tree;

    let len = nodes.len();

    let mut active_sheets: BumpVec<(usize, Stylesheet)> = BumpVec::new_in(temp);
    let mut candidate_rules: BumpVec<usize> = BumpVec::new_in(temp);
    let mut matched_rules: BumpVec<(usize, usize)> = BumpVec::new_in(temp);
    let mut var_ctx = VariableContext::new();
    let mut path: BumpVec<usize> = BumpVec::new_in(temp);
    let mut scratch = ApplyScratch::default();
    let mut affects_layout = false;

    let mut dirty_i: usize = 0;
    while dirty_i < dirty_roots.len() {
        let root_idx = dirty_roots[dirty_i];
        let mut idx: usize = root_idx;
        let mut region_end = idx + nodes[idx].subtree_size + 1;

        active_sheets.clear();
        ancestor_classes.clear();
        var_ctx.clear();
        candidate_rules.clear();

        // ---------- Rebuild Ancestor State ----------
        path.clear();
        let mut curr = idx;
        while curr != usize::MAX {
            // Build idx -> root
            path.push(curr);
            curr = nodes[curr].parent;
        }

        // Apply root -> idx
        for &node_idx in path.iter().rev() {
            if let Some(sheet) = &nodes[node_idx].style_sheet {
                active_sheets.push((node_idx, sheet.clone()));
            }

            if node_idx < var_scope_cache.len() {
                var_ctx.push_vars(node_idx, &var_scope_cache[node_idx]);
            }

            // Collect ancestor pseudo-variables
            for (_, (_, sheet)) in active_sheets.iter().enumerate().rev() {
                candidate_rules.clear();

                let inner = sheet.inner.read();

                candidate_rules.extend_from_slice(&inner.wildcard);
                for &class_id in nodes[node_idx].classes.iter() {
                    if let Some(indexes) = inner.index.get(&class_id) {
                        candidate_rules.extend_from_slice(indexes);
                    }
                }

                candidate_rules.sort_unstable();
                candidate_rules.dedup();

                for &rule_idx in candidate_rules.iter() {
                    let rule = &inner.rules[rule_idx];

                    if !rule.has_pseudos || rule.variables.is_empty() {
                        continue;
                    }

                    if rule_matches_node(rule, nodes, node_idx, focused_node, active_node, hot_nodes, ancestor_classes, None) {
                        var_ctx.push_vars(node_idx, &rule.variables);
                    }
                }
            }

            // Now make this node an ancestor for subsequent nodes in the path.
            if node_idx != idx {
                for &class_id in nodes[node_idx].classes.iter() {
                    if let Err(error) = ancestor_classes.insert_duplicated(class_id) {
                        error!("Selector matching: {error}");
                    }
                }
            }
        }

        // ---------- Style Dirty Region ----------
        let mut first_in_region = true;
        loop {
            // Consume any dirty roots inside the current region,
            // expanding region_end for subtree dirties and on_callback
            while dirty_i < dirty_roots.len() && dirty_roots[dirty_i] < region_end {
                let i = dirty_roots[dirty_i];
                dirty_i += 1;

                let subtree_end = i + nodes[i].subtree_size + 1;
                region_end = region_end.max(subtree_end);
            }
            if idx >= region_end || idx >= len {
                break;
            }

            if !first_in_region {
                // Maintain state incrementally
                update_sheets(nodes, idx, &mut active_sheets);
                update_ancestors(nodes, idx, ancestor_classes);

                let parent_idx = nodes[idx].parent;
                var_ctx.prune(parent_idx);
                if idx < var_scope_cache.len() {
                    var_ctx.push_vars(idx, &var_scope_cache[idx]);
                }
            }

            let subtree_end = idx + nodes[idx].subtree_size + 1;

            let parent_style_snapshot = (idx != 0).then(|| style_cache[nodes[idx].parent].clone());
            let parent_style_opt = parent_style_snapshot.as_ref();

            let mut new_style = if let Some(parent_style) = parent_style_opt {
                Style {
                    color: parent_style.color,
                    font_width: parent_style.font_width,
                    font_size: parent_style.font_size,
                    font_style: parent_style.font_style,
                    font_family: parent_style.font_family.clone(),
                    font_weight: parent_style.font_weight,
                    text_shadow: parent_style.text_shadow.clone(),
                    letter_spacing: parent_style.letter_spacing,
                    word_spacing: parent_style.word_spacing,
                    line_height: parent_style.line_height,
                    ..Default::default()
                }
            } else {
                Style::default()
            };

            // ---------- Collect Rules ----------
            matched_rules.clear();

            // Cache read guards to avoid repeatedly lock/unlocking the same stylesheet inner
            let sheet_inners: SmallVec<[RwLockReadGuard<'_, StylesheetInner>; 8]> = active_sheets.iter().map(|(_, sheet)| sheet.inner.read()).collect();

            for (sheet_stack_idx, _) in active_sheets.iter().enumerate().rev() {
                candidate_rules.clear();

                let inner = &sheet_inners[sheet_stack_idx];

                candidate_rules.extend_from_slice(&inner.wildcard);
                for &class_id in nodes[idx].classes.iter() {
                    if let Some(indexes) = inner.index.get(&class_id) {
                        candidate_rules.extend_from_slice(indexes);
                    }
                }

                candidate_rules.sort_unstable();
                candidate_rules.dedup();

                for &rule_idx in candidate_rules.iter() {
                    let rule = &inner.rules[rule_idx];

                    if rule_matches_node(rule, nodes, idx, focused_node, active_node, hot_nodes, ancestor_classes, None) {
                        matched_rules.push((sheet_stack_idx, rule_idx));

                        if rule.has_pseudos && !rule.variables.is_empty() {
                            var_ctx.push_vars(idx, &rule.variables);
                        }
                    }
                }
            }

            // ---------- Apply properties ----------
            for phase in 0..2 {
                for &(sheet_stack_idx, rule_idx) in matched_rules.iter() {
                    let inner = &sheet_inners[sheet_stack_idx];
                    let rule = &inner.rules[rule_idx];

                    for property in rule.properties.iter() {
                        // apply color first so currentColor resolves against final computed color.
                        let is_color = matches!(property, Property::Color(_));
                        match phase {
                            0 if !is_color => continue,
                            1 if is_color => continue,
                            _ => {}
                        }
                        if phase == 1 {
                            affects_layout |= property.affects_layout();
                        }
                        if let Err(e) = property.apply(&mut scratch, &mut new_style, parent_style_opt, &var_ctx) {
                            css::log_error(&e, e.location, inner.info.as_ref().map(|i| i.path.as_path()));
                        }
                    }
                }
            }

            // ---------- Call on_style Callback ----------
            let mut callback_ran = false;
            if let Some(callback) = nodes[idx].style_callback.as_deref() {
                let prev_layout_style = new_style.get_layout_style();

                let base = on_style_deps.remove(&idx).unwrap_or_default();
                let deps = base.cleared().read_scope(|| {
                    callback(state, &mut new_style);
                });
                on_style_deps.insert(idx, deps);

                affects_layout |= new_style.get_layout_style() != prev_layout_style;
                callback_ran = true;
            }

            // Commit
            style_cache[idx] = new_style;

            if callback_ran {
                region_end = region_end.max(subtree_end);
            }

            idx += 1;
            first_in_region = false;
        }
    }

    dirty_roots.clear();
    affects_layout
}

fn update_sheets<S, H>(nodes: &[Node<S, H>], idx: usize, active_sheets: &mut BumpVec<(usize, Stylesheet)>) {
    // If this stylesheet came from a node after this node's parent, it can't apply.
    while active_sheets.pop_if(|(style_idx, _)| *style_idx > nodes[idx].parent).is_some() {}

    if let Some(stylesheet) = &nodes[idx].style_sheet {
        active_sheets.push((idx, stylesheet.clone()));
    }
}

fn update_ancestors<S, H>(nodes: &[Node<S, H>], idx: usize, ancestor_classes: &mut Filter) {
    if idx != 0 {
        let parent = nodes[idx].parent;
        let prev_parent = nodes[idx - 1].parent;

        // Sibling node, ancestors haven't changed
        if parent == prev_parent {
            return;
        }

        // Moved down tree
        if parent > prev_parent || prev_parent == usize::MAX {
            // Add ancestors
            for &class_id in nodes[parent].classes.iter() {
                if let Err(error) = ancestor_classes.insert_duplicated(class_id) {
                    error!("Selector matching: {error}");
                }
            }

        // Moved up tree
        } else {
            // Remove inapplicable ancestors
            let mut curr = prev_parent;
            while curr != parent {
                for &class_id in nodes[curr].classes.iter() {
                    ancestor_classes.remove(class_id);
                }
                curr = nodes[curr].parent;
            }
        }
    }
}

/// Checks if a rule matches a node in the tree.
/// When pseudo_out is None, it checks the actual pseudo state.
/// When pseudo out is Some, it fills the vec with the pseudo classes that would match this node.
#[allow(clippy::too_many_arguments)]
fn rule_matches_node<S, H>(
    rule: &Rule,
    nodes: &[Node<S, H>],
    idx: usize,
    focused_node: Option<NodeId>,
    active_node: Option<NodeId>,
    hot_nodes: &[usize],
    ancestor_classes: &Filter,
    mut pseudo_out: Option<&mut Vec<(PseudoKind, usize)>>,
) -> bool {
    let mut cmp_node = idx;
    let mut is_first = true;
    let mut prev_class = false;
    let mut prev_child = false;

    'selector: for selector in rule.selectors.iter().rev() {
        'node: while cmp_node != usize::MAX {
            match selector {
                Selector::Class(rule_class_id) => {
                    if nodes[cmp_node].classes.contains(rule_class_id) {
                        is_first = false;
                        prev_class = true;
                        prev_child = false;
                        continue 'selector;
                    } else if is_first || prev_class || prev_child {
                        return false;
                    }

                    if !ancestor_classes.contains(rule_class_id) {
                        return false;
                    }

                    is_first = false;
                    prev_class = true;
                    prev_child = false;

                    cmp_node = nodes[cmp_node].parent;
                    continue 'node;
                }

                Selector::Wildcard => {
                    is_first = false;
                    prev_class = false;
                    prev_child = false;
                    continue 'selector;
                }

                Selector::Child => {
                    prev_class = false;
                    prev_child = true;
                    cmp_node = nodes[cmp_node].parent;
                    continue 'selector;
                }

                Selector::Descendant => {
                    prev_child = false;
                    prev_class = false;
                    cmp_node = nodes[cmp_node].parent;
                    continue 'selector;
                }
                Selector::Hover => {
                    if let Some(out) = pseudo_out.as_deref_mut() {
                        out.push((PseudoKind::Hover, cmp_node));
                        prev_child = false;
                        prev_class = false;
                        continue 'selector;
                    } else if hot_nodes.contains(&cmp_node) {
                        prev_child = false;
                        prev_class = false;
                        continue 'selector;
                    } else {
                        return false;
                    }
                }
                Selector::Focus => {
                    if let Some(out) = pseudo_out.as_deref_mut() {
                        out.push((PseudoKind::Focus, cmp_node));
                        prev_child = false;
                        prev_class = false;
                        continue 'selector;
                    } else if let (Some(cmp_nid), Some(focus_nid)) = (nodes[cmp_node].nid, focused_node)
                        && cmp_nid == focus_nid
                    {
                        prev_child = false;
                        prev_class = false;
                        continue 'selector;
                    } else {
                        return false;
                    }
                }
                Selector::Active => {
                    if let Some(out) = pseudo_out.as_deref_mut() {
                        out.push((PseudoKind::Active, cmp_node));
                        prev_child = false;
                        prev_class = false;
                        continue 'selector;
                    } else if let (Some(cmp_nid), Some(active_nid)) = (nodes[cmp_node].nid, active_node)
                        && cmp_nid == active_nid
                    {
                        prev_child = false;
                        prev_class = false;
                        continue 'selector;
                    } else {
                        return false;
                    }
                }
                Selector::Enabled | Selector::Disabled => {
                    if let Some(out) = pseudo_out.as_deref_mut() {
                        out.push((PseudoKind::Enabled, cmp_node));
                        prev_child = false;
                        prev_class = false;
                        continue 'selector;
                    } else {
                        let enabled = nodes[cmp_node].enabled.get().unwrap_or(true);
                        let want_enabled = matches!(selector, Selector::Enabled);
                        if enabled == want_enabled {
                            prev_child = false;
                            prev_class = false;
                            continue 'selector;
                        }
                        return false;
                    }
                }
            }
        }
        return false;
    }
    true
}

#[cfg(feature = "serde")]
impl serde::Serialize for Stylesheet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let inner = self.inner.read();

        let mut map = serializer.serialize_map(Some(1))?;
        if let Some(info) = &inner.info {
            let path = info.path.to_string_lossy();
            map.serialize_entry("path", path.as_ref())?;
        } else {
            let data = self.to_string();
            map.serialize_entry("css", data.as_str())?;
        }
        map.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Stylesheet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct StylesheetVisitor;

        impl<'de> Visitor<'de> for StylesheetVisitor {
            type Value = Stylesheet;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a stylesheet string or filename")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut path: Option<String> = None;
                let mut css: Option<String> = None;

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "path" => {
                            if path.is_some() {
                                return Err(de::Error::duplicate_field("path"));
                            }
                            path = Some(map.next_value()?);
                        }
                        "css" => {
                            if css.is_some() {
                                return Err(de::Error::duplicate_field("css"));
                            }
                            css = Some(map.next_value()?);
                        }
                        other => {
                            // Unknown key: consume the value so we can keep parsing.
                            let _ = map.next_value::<de::IgnoredAny>()?;
                            return Err(de::Error::unknown_field(other, &["path", "css"]));
                        }
                    }
                }

                match (path, css) {
                    (Some(_), Some(_)) => Err(de::Error::custom("expected exactly one of \"path\" or \"css\"")),
                    (Some(p), None) => Stylesheet::from_file(p).map_err(de::Error::custom),
                    (None, Some(c)) => Stylesheet::from_str(&c).map_err(|_| de::Error::custom("failed to parse stylesheet CSS")),
                    (None, None) => Err(de::Error::custom("missing \"path\" or \"css\"")),
                }
            }
        }

        deserializer.deserialize_map(StylesheetVisitor)
    }
}
