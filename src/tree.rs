#![forbid(unsafe_code)]

use crate::prelude::*;

use std::num::NonZeroUsize;

/// Macro for describing the structure and style of a UI
///
/// [] - Create and set classes on a new node
/// () - Set classes on interior instead of creating a new node
/// {} - Call methods on parent node
#[macro_export]
macro_rules! ui {
    ($classes:literal [ $($children:tt)* ]) => {
        ui!(Node::new() .add_classes($classes); $($children)* )
    };
    ($tree:expr; $classes:literal [ $($children:tt)* ] $($tail:tt)*) => {
        ui!($tree.add_child(ui!(Node::new() .add_classes($classes); $($children)* )); $($tail)* )
    };
    ($tree:expr; $classes:literal ( $($child:tt)* ) $($tail:tt)*) => {
        ui!($tree.add_child($($child)* .add_classes($classes) ); $($tail)* )
    };
    ($tree:expr; $classes:literal { $($builder:tt)* } $($tail:tt)*) => {
        ui!($tree.add_child(Node::new() .add_classes($classes) $($builder)* ); $($tail)* )
    };
    ($tree:expr; { $($body:tt)* } $($tail:tt)*) => {
        ui!($tree $($body)*; $($tail)* )
    };

    // Control flow
    ($tree:expr; if let $v:pat = $e:tt $($tail:tt)*) => {
        ui!(@munch; @if_block; $tree; tree; (if let $v = $e), $($tail)*)
    };
    ($tree:expr; if $e:tt $($tail:tt)*) => {
        ui!(@munch; @if_block; $tree; tree; (if $e), $($tail)*)
    };
    ($tree:expr; for $v:pat in $e:tt $($tail:tt)*) => {
        ui!(@munch; @for_block; $tree; tree; (for $v in $e), $($tail)*)
    };
    ($tree:expr; match $e:tt $($tail:tt)*) => {
        ui!(@munch; @match_block; $tree; tree; (match $e), $($tail)*)
    };

    // If chains
    (@if_block; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($body:tt)* } else if $e:tt $($tail:tt)*) => {
        ui!(@munch; if_block; $tree; $temp; ($($prefix)* { $temp = ui!($temp; $($body)* ); } else if $e), $($tail)* )
    };
    (@if_block; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($one:tt)* } else { $($two:tt)* } $($tail:tt)*) => {
        ui!({ let mut $temp = $tree; $($prefix)* { $temp = ui!($temp; $($one)* ); } else { $temp = ui!($temp; $($two)* ); } $temp }; $($tail)* )
    };
    (@if_block; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($body:tt)* } $($tail:tt)*) => {
        ui!({ let mut $temp = $tree; $($prefix)* { $temp = ui!($temp; $($body)* ); } $temp }; $($tail)* )
    };

    // Match block
    (@match_block; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($pattern:pat => { $($branch:tt)* } $(,)?)* } $($tail:tt)*) => {
        ui!({ let mut $temp = $tree; $temp = $($prefix)* { $($pattern => {ui!($temp; $($branch)*)} )* }; $temp }; $($tail)* )
    };

    // For loop
    (@for_block; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($body:tt)* } $($tail:tt)*) => {
        ui!({ let mut $temp = $tree; $($prefix)* { $temp = ui!($temp; $($body)* ); } $temp }; $($tail)* )
    };

    // Prefix muncher
    (@munch; @$goto:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($body:tt)* } $($tail:tt)*) => {
        ui!(@$goto; $tree; $temp; ($($prefix)*), { $($body)* } $($tail)* )
    };
    (@munch; @$goto:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), $first:tt $($tail:tt)*) => {
        ui!(@munch; @$goto; $tree; $temp; ($($prefix)* $first), $($tail)* )
    };

    // Default case
    ($tree:expr; $($tail:tt)*) => {
        $tree $($tail)*
    };
}

// TODO - replace with on_draw() callback. Labels, Images, etc. should just be widgets
pub enum Content<'a, T> {
    None,
    Label(&'a str),
    DynamicLabel(&'a dyn Fn(&'a T) -> &'a str),
}

impl<'a, T> Default for Content<'a, T> {
    fn default() -> Self {
        Self::None
    }
}

pub(crate) struct ArrayNode<T: 'static> {
    pub _key: Option<Key>, // TODO
    pub classes: Vec<&'static str>,
    pub _callbacks: Vec<(On, Box<dyn EventCallback<T>>)>, // TODO
    pub style: Style,
    pub style_on_draw: Option<Box<dyn StyleCallback<T>>>,
    pub parent: usize,
    pub num_children: usize,
    pub last_child: Option<NonZeroUsize>,
    pub content: Content<'static, T>,
}

impl<T> ArrayNode<T> {
    // Note: Children are reversed
    pub(crate) fn child_ids(&self) -> std::ops::Range<usize> {
        if let Some(last_child) = self.last_child {
            last_child.get()..(last_child.get() + self.num_children)
        } else {
            0..0
        }
    }

    // TODO
    pub(crate) fn _trigger(&self, event_type: On, state: &'static mut T, app: &mut App<T>) -> Stage {
        let mut stage = Stage::Idle;
        for (et, callback) in &self._callbacks {
            if *et == event_type {
                stage = stage.max((callback)(state, app));
            }
        }
        stage
    }
}

pub struct Node<T: 'static> {
    key: Option<Key>,
    classes: Option<Vec<&'static str>>,
    callbacks: Option<Vec<(On, Box<dyn EventCallback<T>>)>>,
    style_default: Option<fn() -> Style>,
    style_on_draw: Option<Box<dyn StyleCallback<T>>>,
    size: usize,
    num_children: usize,
    prev_sibling: Option<Box<Node<T>>>,
    last_child: Option<Box<Node<T>>>,
    content: Content<'static, T>,
}

impl<T> Node<T> {
    pub fn new() -> Self {
            Self {
                key: None,
                classes: Some(Vec::new()),
                callbacks: Some(Vec::new()),
                style_default: None,
                style_on_draw: None,
                size: 1,
                num_children: 0,
                prev_sibling: None,
                last_child: None,
                content: Content::None,
            }
    }

    /// Set a key on a node, providing a stable identity between rebuilds
    pub fn key(mut self, key: Key) -> Self {
        self.key = Some(key);
        self
    }

    /// Add CSS classes
    pub fn add_classes(mut self, classes: &'static str) -> Self {
        if let Some(class_vec) = &mut self.classes {
            for class in classes.split_whitespace() {
                class_vec.push(class);
            }
        }
        self
    }

    /// Register an event callback
    pub fn event(mut self, event_type: On, callback: impl EventCallback<T>) -> Self {
        if let Some(callbacks) = &mut self.callbacks {
            callbacks.push((event_type, Box::new(callback)));
        }
        self
    }

    /// Register a function that will provide an alternate default Style for this node
    pub fn style_default(mut self, func: fn() -> Style) -> Self {
        self.style_default = Some(func);
        self
    }

    /// Register a function to modify this node's style right before redrawing
    pub fn style_on_draw(mut self, func: impl StyleCallback<T>) -> Self {
        self.style_on_draw = Some(Box::new(func));
        self
    }

    /// Set the contents of a node
    pub fn content(mut self, content: Content<'static, T>) -> Self {
        self.content = content;
        self
    }

    /// Add a child node
    pub fn add_child(mut self, mut new_child: Self) -> Self {
        self.size += new_child.size;
        self.num_children += 1;

        if let Some(last_child) = self.last_child {
            new_child.size += last_child.size;
            new_child.prev_sibling = Some(last_child);
        }

        self.last_child = Some(Box::new(new_child));
        self
    }

    pub(crate) fn finish(self) -> Option<Vec<ArrayNode<T>>> {
        let mut tree: Vec<ArrayNode<T>> = Vec::with_capacity(self.size);
        let mut stack: Vec<(bool, usize, Box<Node<T>>)> = Vec::new();

        stack.push((false, 0, Box::new(self)));
        while let Some((is_last_child, parent, mut curr_node)) = stack.pop() {
            let index = tree.len();
            if is_last_child {
                tree[parent].last_child = NonZeroUsize::new(index);
            }

            tree.push(ArrayNode {
                _key: curr_node.key,
                classes: curr_node.classes.take()?,
                _callbacks: curr_node.callbacks.take()?,
                style: curr_node.style_default.unwrap_or(Style::default)(),
                style_on_draw: curr_node.style_on_draw.take(),
                parent,
                num_children: curr_node.num_children,
                last_child: None,
                content: std::mem::take(&mut curr_node.content),
            });

            if let Some(last_child) = curr_node.last_child.take() {
                stack.push((true, index, last_child));
            }

            if let Some(prev_sibling) = curr_node.prev_sibling.take() {
                stack.push((false, parent, prev_sibling));
            }
        }

        Some(tree)
    }
}
