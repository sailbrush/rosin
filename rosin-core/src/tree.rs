//! Exposes the builder used to construct a UI tree.

use std::{collections::HashMap, fmt, panic::Location, sync::Arc};

use bumpalo::collections::Vec as BumpVec;
use kurbo::{RoundedRect, Size, Vec2};

use crate::{
    hasher::IdentityBuildHasher,
    interner::{StrId, StringInterner},
    layout::TextCacheEntry,
    prelude::*,
};

type EventCallback<S, H> = Box<dyn Fn(&mut S, &mut EventCtx<H>)>;
type StyleCallback<S> = Box<dyn Fn(&S, &mut Style)>;
type MeasureCallback<S> = Box<dyn Fn(&S, &MeasureCtx) -> Size>;
type CanvasCallback<S> = Box<dyn Fn(&S, &mut CanvasCtx)>;
type AccessibilityCallback<S> = Arc<dyn for<'a> Fn(&S, &mut AccessibilityCtx<'a>) + Send + Sync + 'static>;

pub(crate) struct Node<S: 'static, H> {
    pub nid: Option<NodeId>,
    pub classes: Vec<StrId>,
    pub text: Option<UIString>,
    pub enabled: UIParam<bool>,
    pub offset: Option<UIParam<Vec2>>,
    pub event_callbacks: Vec<(On, EventCallback<S, H>)>,
    pub style_sheet: Option<Stylesheet>,
    pub style_callback: Option<StyleCallback<S>>,
    pub measure_callback: Option<MeasureCallback<S>>,
    pub canvas_callback: Option<CanvasCallback<S>>,
    pub accessibility_callback: Option<AccessibilityCallback<S>>,
    pub parent: usize,
    pub num_children: usize,
    pub subtree_size: usize,
}

impl<S, H> fmt::Debug for Node<S, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.nid)
            .field("classes", &self.classes)
            .field("text", &self.text.is_some())
            .field("enabled", &self.enabled.get())
            .field("offset", &self.offset.as_ref().map(|p| p.get()))
            .field("event_callbacks", &self.event_callbacks.len())
            .field("style_sheet", &self.style_sheet.is_some())
            .field("style_callback", &self.style_callback.is_some())
            .field("canvas_callback", &self.canvas_callback.is_some())
            .field("accessibility_callback", &self.accessibility_callback.is_some())
            .field("parent", &self.parent)
            .field("num_children", &self.num_children)
            .field("subtree_size", &self.subtree_size)
            .finish()
    }
}

impl<S, H> Node<S, H> {
    pub fn run_callbacks(&mut self, event_type: On, state: &mut S, ctx: &mut EventCtx<H>) -> bool {
        let mut has_callback = false;
        for (et, callback) in self.event_callbacks.iter_mut() {
            if *et == event_type {
                (callback)(state, ctx);
                has_callback = true;
            }
        }
        has_callback
    }

    pub fn has_callback(&self, event_type: On) -> bool {
        for (et, _) in self.event_callbacks.iter() {
            if *et == event_type {
                return true;
            }
        }
        false
    }
}

/// Used to construct a UI tree with the builder pattern.
pub struct Ui<S: 'static, H: 'static> {
    /// The main array of tree nodes
    pub(crate) nodes: Vec<Node<S, H>>,

    /// The indexes of nodes that have On::AnimationFrame callbacks
    pub(crate) on_anim_nodes: Vec<usize>,

    /// The indexes of nodes that have on_style callbacks
    pub(crate) on_style_nodes: Vec<usize>,

    /// Dependency maps for each node that has a style callback
    pub(crate) on_style_deps: HashMap<usize, DependencyMap, IdentityBuildHasher>,

    /// The indexes of nodes that have a dynamic enabled UIParam, and the previous value
    pub(crate) dynamic_enabled_prev: Vec<(usize, bool)>,

    /// The indexes of nodes with the `position: fixed` property
    pub(crate) fixed_nodes: Vec<usize>,

    /// A map from NodeIds to indexes in the `nodes` array
    pub(crate) nid_map: HashMap<NodeId, usize, IdentityBuildHasher>,

    /// Sorted list of NodeIds currently present in `nid_map`
    pub(crate) nid_sorted: Vec<NodeId>,

    /// The computed styles for the tree
    pub(crate) style_cache: Vec<Style>,

    /// The computed style flags for the tree
    pub(crate) style_flags: Vec<u8>,

    /// Style dirty flags for the tree
    pub(crate) dirty_roots: Vec<usize>,

    /// New variables introduced by static rules on each node
    pub(crate) var_scope_cache: Vec<Vec<(Arc<str>, Arc<str>)>>,

    /// The computed layouts for the tree. RounededRect because it affects hit-testing.
    pub(crate) layout_cache: Vec<RoundedRect>,

    /// The computed text layouts for the tree
    pub(crate) text_cache: HashMap<usize, TextCacheEntry, IdentityBuildHasher>,

    /// The max number of children that any node in the tree has
    pub(crate) max_children: usize,

    /// The parent that we're currently adding children to
    current_parent_idx: usize,

    /// The node that we're currently applying properties to
    current_node_idx: usize,
}

impl<S: 'static, H: 'static> fmt::Debug for Ui<S, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ui")
            .field("nodes", &self.nodes)
            .field("on_anim_nodes", &self.on_anim_nodes)
            .field("on_style_nodes", &self.on_style_nodes)
            .field("on_style_deps", &self.on_style_deps)
            .field("dynamic_enabled_prev", &self.dynamic_enabled_prev)
            .field("fixed_nodes", &self.fixed_nodes)
            .field("nid_map", &self.nid_map)
            .field("nid_sorted", &self.nid_sorted)
            .field("style_cache", &self.style_cache)
            .field("style_flags", &self.style_flags)
            .field("dirty_roots", &self.dirty_roots)
            .field("var_scope_cache", &self.var_scope_cache)
            .field("layout_cache", &self.layout_cache)
            .field("text_cache", &self.text_cache)
            .field("max_children", &self.max_children)
            .field("current_parent_idx", &self.current_parent_idx)
            .field("current_node_idx", &self.current_node_idx)
            .finish()
    }
}

impl<S, H> Ui<S, H> {
    pub(crate) fn new() -> Self {
        Ui {
            nodes: Vec::with_capacity(1000),
            on_anim_nodes: Vec::new(),
            on_style_nodes: Vec::new(),
            on_style_deps: HashMap::with_hasher(IdentityBuildHasher),
            dynamic_enabled_prev: Vec::new(),
            fixed_nodes: Vec::new(),
            nid_map: HashMap::with_hasher(IdentityBuildHasher),
            nid_sorted: Vec::new(),
            style_cache: Vec::new(),
            style_flags: Vec::new(),
            dirty_roots: Vec::new(),
            var_scope_cache: Vec::new(),
            layout_cache: Vec::new(),
            text_cache: HashMap::with_hasher(IdentityBuildHasher),
            max_children: 0,
            current_parent_idx: usize::MAX,
            current_node_idx: usize::MAX,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.nodes.clear();
        self.on_anim_nodes.clear();
        self.on_style_nodes.clear();
        self.on_style_deps.clear();
        self.dynamic_enabled_prev.clear();
        self.fixed_nodes.clear();
        self.nid_map.clear();
        self.nid_sorted.clear();
        self.style_cache.clear();
        self.style_flags.clear();
        self.dirty_roots.clear();
        self.var_scope_cache.clear();
        self.layout_cache.clear();
        self.text_cache.clear();
        self.max_children = 0;
        self.current_parent_idx = usize::MAX;
        self.current_node_idx = usize::MAX;
    }

    pub(crate) fn finish(&mut self) {
        self.nid_sorted.clear();
        self.nid_sorted.reserve_exact(self.nid_map.len());
        self.nid_sorted.extend(self.nid_map.keys().copied());
        self.nid_sorted.sort_unstable();

        self.on_anim_nodes.sort_unstable();
        self.on_anim_nodes.dedup();

        self.on_style_nodes.sort_unstable();
        self.on_style_nodes.dedup();

        let len = self.nodes.len();
        self.style_cache.resize(len, Style::default());
        self.style_flags.reserve_exact(len);
        self.var_scope_cache.reserve_exact(len);

        self.dynamic_enabled_prev.sort_unstable();
        self.dynamic_enabled_prev.dedup();
        for (idx, value) in &mut self.dynamic_enabled_prev {
            *value = self.nodes[*idx].enabled.get_or(true);
        }

        self.dirty_roots.push(0);
    }

    /// Get a list of children in tree-order
    pub(crate) fn child_indexes(&self, parent: usize, output: &mut BumpVec<usize>) {
        output.clear();
        let count = self.nodes[parent].num_children;
        if count == 0 {
            return;
        }
        output.reserve(count);

        // Skip each child's subtree
        let mut idx = parent + 1;
        let end = idx + self.nodes[parent].subtree_size;
        while idx < end {
            output.push(idx);
            idx += self.nodes[idx].subtree_size + 1;
        }
    }

    /// Accepts *_DIRTY flags, and creates a dirty root if the node responds to those flags.
    ///
    /// Returns true if a root was created.
    pub(crate) fn add_dirty_root_by_nid(&mut self, nid: Option<NodeId>, flag: u8) -> bool {
        let Some(nid) = nid else {
            return false;
        };
        let Some(&idx) = self.nid_map.get(&nid) else {
            return false;
        };
        self.add_dirty_root_by_idx(idx, flag)
    }

    /// Accepts *_DIRTY flags, and creates a dirty root if the node responds to those flags.
    ///
    /// Returns true if a root was created.
    pub(crate) fn add_dirty_root_by_idx(&mut self, idx: usize, flag: u8) -> bool {
        if (self.style_flags[idx] & flag) != 0 {
            self.dirty_roots.push(idx);
            true
        } else {
            false
        }
    }

    /// Sorts and merges the list of dirty roots.
    pub(crate) fn merge_dirty_roots(&mut self) {
        self.dirty_roots.sort_unstable();

        let mut kept = 0usize; // count of roots we've kept
        let mut dirty_until = 0usize;

        for r in 0..self.dirty_roots.len() {
            let idx = self.dirty_roots[r];

            // Dedup exact duplicates
            if kept != 0 && self.dirty_roots[kept - 1] == idx {
                continue;
            }

            // Skip roots covered by a previously kept dirty subtree
            if idx < dirty_until {
                continue;
            }

            self.dirty_roots[kept] = idx;
            kept += 1;

            dirty_until = idx + self.nodes[idx].subtree_size + 1;
        }

        self.dirty_roots.truncate(kept);
    }

    /// Create a new node in the tree and make it the current node for subsequent builder calls to configure.
    ///
    /// **Panics**: If called a second time at the root level (only one root node is allowed).
    #[track_caller]
    #[inline]
    pub fn node(&mut self) -> &mut Self {
        debug_assert!(self.current_parent_idx != usize::MAX || self.nodes.is_empty(), "There can only be one root node.");

        if self.current_parent_idx != usize::MAX {
            let parent = &mut self.nodes[self.current_parent_idx];
            parent.num_children += 1;
        }

        self.current_node_idx = self.nodes.len();

        self.nodes.push(Node {
            nid: None,
            classes: Vec::new(),
            text: None,
            enabled: UIParam::Static(true),
            offset: None,
            event_callbacks: Vec::new(),
            style_sheet: None,
            style_callback: None,
            measure_callback: None,
            canvas_callback: None,
            accessibility_callback: None,
            parent: self.current_parent_idx,
            num_children: 0,
            subtree_size: 0,
        });

        self
    }

    /// Assign an id to the current node, providing a stable identity between build phases.
    ///
    /// **Panics**: If the ID has already been assigned to a different node.
    #[track_caller]
    #[inline]
    pub fn id(&mut self, id: NodeId) -> &mut Self {
        debug_assert!(self.current_node_idx != usize::MAX, "You must call .node() before setting properties.");
        let current_node = &mut self.nodes[self.current_node_idx];

        if let Some(old_id) = current_node.nid {
            self.nid_map.remove(&old_id);
        }

        current_node.nid = Some(id);

        if self.nid_map.insert(id, self.current_node_idx).is_some() {
            let location = Location::caller();
            panic!("NodeId reused at {location}.");
        }

        self
    }

    /// Append a space-separated list of CSS classes to the current node.
    ///
    /// You can pass in `None` to remove all classes from the current node.
    #[track_caller]
    #[inline]
    pub fn classes<'a>(&mut self, classes: impl Into<Option<&'a str>>) -> &mut Self {
        debug_assert!(self.current_node_idx != usize::MAX, "You must call .node() before setting properties.");
        let current_node = &mut self.nodes[self.current_node_idx];
        let classes = classes.into();
        if let Some(classes) = classes {
            let mut interner = StringInterner::global().write();
            for class in classes.split_whitespace() {
                let str_id = interner.intern(class);
                if !current_node.classes.contains(&str_id) {
                    current_node.classes.push(str_id);
                }
            }
        } else {
            current_node.classes.clear();
        }
        self
    }

    /// Sets the text to be rendered inside this node.
    #[track_caller]
    #[inline]
    pub fn text(&mut self, text: impl Into<UIString>) -> &mut Self {
        debug_assert!(self.current_node_idx != usize::MAX, "You must call .node() before setting properties.");
        let current_node = &mut self.nodes[self.current_node_idx];
        current_node.text = Some(text.into());
        self
    }

    /// Sets whether the current node is enabled. Disabled nodes do not receive events and may be styled differently.
    ///
    /// Accepts `impl Into<UIParam>` so you can pass a reactive [`Var`] or a constant [`bool`] directly.
    #[track_caller]
    #[inline]
    pub fn enabled(&mut self, enabled: impl Into<UIParam<bool>>) -> &mut Self {
        debug_assert!(self.current_node_idx != usize::MAX, "You must call .node() before setting properties.");
        let enabled = enabled.into();
        if let UIParam::Dynamic(_) = enabled {
            // we can't get the current value now because reading it would register it as a build phase dependency
            self.dynamic_enabled_prev.push((self.current_node_idx, true));
        }
        self.nodes[self.current_node_idx].enabled = enabled;
        self
    }

    /// Provide an offset value to change the position of child nodes. Useful for scroll areas.
    ///     
    /// Accepts `impl Into<UIParam>` so you can pass a reactive [`Var`] or a constant [`Vec2`] directly.
    #[track_caller]
    #[inline]
    pub fn offset(&mut self, offset: impl Into<UIParam<Vec2>>) -> &mut Self {
        debug_assert!(self.current_node_idx != usize::MAX, "You must call .node() before setting properties.");
        self.nodes[self.current_node_idx].offset = Some(offset.into());
        self
    }

    /// Apply a stylesheet to the current node and all of its children.
    ///
    /// You can pass in `None` to remove a stylesheet from the current node. There can only be one stylesheet per node.
    #[track_caller]
    #[inline]
    pub fn style_sheet<'a>(&mut self, style_sheet: impl Into<Option<&'a Stylesheet>>) -> &mut Self {
        debug_assert!(self.current_node_idx != usize::MAX, "You must call .node() before setting properties.");
        self.nodes[self.current_node_idx].style_sheet = style_sheet.into().cloned();
        self
    }

    /// Register an event callback.
    #[track_caller]
    #[inline]
    pub fn event(&mut self, event_type: On, callback: impl Fn(&mut S, &mut EventCtx<H>) + 'static) -> &mut Self {
        debug_assert!(self.current_node_idx != usize::MAX, "You must call .node() before setting properties.");
        if event_type == On::AnimationFrame {
            self.on_anim_nodes.push(self.current_node_idx);
        }
        self.nodes[self.current_node_idx].event_callbacks.push((event_type, Box::new(callback)));
        self
    }

    /// Register a function to modify the current node's style before the layout and draw phases.
    /// Useful for applying the results of an [`On::AnimationFrame`] event handler.
    ///
    /// There can only be one `on_style` callback per node.
    #[track_caller]
    #[inline]
    pub fn on_style(&mut self, callback: impl Fn(&S, &mut Style) + 'static) -> &mut Self {
        debug_assert!(self.current_node_idx != usize::MAX, "You must call .node() before setting properties.");
        self.on_style_nodes.push(self.current_node_idx);
        self.nodes[self.current_node_idx].style_callback = Some(Box::new(callback));
        self
    }

    /// Register a function to calculate the node's preferred size based on the provided constraints.
    /// This overrides the node's built-in text layout measurement.
    ///
    /// There can only be one `on_measure` callback per node.
    #[track_caller]
    #[inline]
    pub fn on_measure(&mut self, callback: impl Fn(&S, &MeasureCtx) -> Size + 'static) -> &mut Self {
        debug_assert!(self.current_node_idx != usize::MAX, "You must call .node() before setting properties.");
        self.nodes[self.current_node_idx].measure_callback = Some(Box::new(callback));
        self
    }

    /// Register a function to draw arbitrary vector graphics inside the current node.
    ///
    /// There can only be one `on_canvas` callback per node.
    #[track_caller]
    #[inline]
    pub fn on_canvas(&mut self, callback: impl Fn(&S, &mut CanvasCtx) + 'static) -> &mut Self {
        debug_assert!(self.current_node_idx != usize::MAX, "You must call .node() before setting properties.");
        self.nodes[self.current_node_idx].canvas_callback = Some(Box::new(callback));
        self
    }

    /// Register a function to construct the accessability information for the current node if the platform requests it.
    ///
    /// There can only be one `on_accessability` callback per node.
    #[track_caller]
    #[inline]
    pub fn on_accessibility(&mut self, f: impl for<'a> Fn(&S, &mut AccessibilityCtx<'a>) + Send + Sync + 'static) -> &mut Self {
        debug_assert!(self.current_node_idx != usize::MAX, "You must call .node() before setting properties.");
        self.nodes[self.current_node_idx].accessibility_callback = Some(std::sync::Arc::new(f));
        self
    }

    /// Add child nodes.
    #[track_caller]
    #[inline]
    pub fn children(&mut self, func: impl FnOnce(&mut Ui<S, H>)) -> &mut Self {
        debug_assert!(self.current_node_idx != usize::MAX, "You must call .node() before setting properties.");
        let parent_index = self.current_parent_idx;
        let node_index = self.current_node_idx;

        self.current_parent_idx = node_index;
        self.current_node_idx = usize::MAX;
        func(self);

        let subtree_size = self.nodes.len() - (1 + node_index);
        self.nodes[node_index].subtree_size = subtree_size;

        self.current_parent_idx = parent_index;
        self.current_node_idx = node_index;
        self.max_children = self.max_children.max(self.nodes[node_index].num_children);

        self
    }
}
