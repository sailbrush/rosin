#![forbid(unsafe_code)]

use crate::prelude::*;

use std::num::NonZeroUsize;

use bumpalo::{collections::Vec as BumpVec, Bump};

/// Macro for describing the structure and style of a UI
///
/// [] - Create and set classes on a new node
/// () - Set classes on interior instead of creating a new node
/// {} - Call methods on parent node
#[macro_export]
macro_rules! ui {
    ($al:ident, $classes:literal [ $($children:tt)* ]) => {
        ui!($al, Node::new_in($al) .add_classes($classes); $($children)* )
    };
    ($al:ident, $tree:expr; $classes:literal [ $($children:tt)* ] $($tail:tt)*) => {
        ui!($al, $tree.add_child($al, ui!($al, Node::new_in($al) .add_classes($classes); $($children)* )); $($tail)* )
    };
    ($al:ident, $tree:expr; $classes:literal ( $($child:tt)* ) $($tail:tt)*) => {
        ui!($al, $tree.add_child($al, $($child)* .add_classes($classes) ); $($tail)* )
    };
    ($al:ident, $tree:expr; $classes:literal { $($builder:tt)* } $($tail:tt)*) => {
        ui!($al, $tree.add_child($al, Node::new_in($al) .add_classes($classes) $($builder)* ); $($tail)* )
    };
    ($al:ident, $tree:expr; { $($body:tt)* } $($tail:tt)*) => {
        ui!($al, $tree $($body)*; $($tail)* )
    };

    // Control flow
    ($al:ident, $tree:expr; if let $v:pat = $e:tt $($tail:tt)*) => {
        ui!(@munch; @if_block; $al; $tree; tree; (if let $v = $e), $($tail)*)
    };
    ($al:ident, $tree:expr; if $e:tt $($tail:tt)*) => {
        ui!(@munch; @if_block; $al; $tree; tree; (if $e), $($tail)*)
    };
    ($al:ident, $tree:expr; for $v:pat in $e:tt $($tail:tt)*) => {
        ui!(@munch; @for_block; $al; $tree; tree; (for $v in $e), $($tail)*)
    };
    ($al:ident, $tree:expr; match $e:tt $($tail:tt)*) => {
        ui!(@munch; @match_block; $al; $tree; tree; (match $e), $($tail)*)
    };

    // If chains
    (@if_block; $al:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($body:tt)* } else if $e:tt $($tail:tt)*) => {
        ui!(@munch; $al; if_block; $tree; $temp; ($($prefix)* { $temp = ui!($al, $temp; $($body)* ); } else if $e), $($tail)* )
    };
    (@if_block; $al:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($one:tt)* } else { $($two:tt)* } $($tail:tt)*) => {
        ui!($al, { let mut $temp = $tree; $($prefix)* { $temp = ui!($al, $temp; $($one)* ); } else { $temp = ui!($al, $temp; $($two)* ); } $temp }; $($tail)* )
    };
    (@if_block; $al:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($body:tt)* } $($tail:tt)*) => {
        ui!($al, { let mut $temp = $tree; $($prefix)* { $temp = ui!($al, $temp; $($body)* ); } $temp }; $($tail)* )
    };

    // Match block
    (@match_block; $al:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($pattern:pat => { $($branch:tt)* } $(,)?)* } $($tail:tt)*) => {
        ui!($al, { let mut $temp = $tree; $temp = $($prefix)* { $($pattern => {ui!($al, $temp; $($branch)*)} )* }; $temp }; $($tail)* )
    };

    // For loop
    (@for_block; $al:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($body:tt)* } $($tail:tt)*) => {
        ui!($al, { let mut $temp = $tree; $($prefix)* { $temp = ui!($al, $temp; $($body)* ); } $temp }; $($tail)* )
    };

    // Prefix muncher
    (@munch; @$goto:ident; $al:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($body:tt)* } $($tail:tt)*) => {
        ui!(@$goto; $al; $tree; $temp; ($($prefix)*), { $($body)* } $($tail)* )
    };
    (@munch; @$goto:ident; $al:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), $first:tt $($tail:tt)*) => {
        ui!(@munch; @$goto; $al; $tree; $temp; ($($prefix)* $first), $($tail)* )
    };

    // Default case
    ($al:ident, $tree:expr; $($tail:tt)*) => {
        $tree $($tail)*
    };
}

// A handle to an allocator
#[derive(Default)]
pub struct Alloc {
    pub(crate) bump: Bump,
}

impl Alloc {
    pub fn alloc<U>(&self, val: U) -> &mut U {
        self.bump.alloc(val)
    }
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

pub(crate) struct ArrayNode<'a, T> {
    pub key: Option<Key>,
    pub classes: BumpVec<'a, &'static str>,
    pub callbacks: BumpVec<'a, (On, &'a EventCallback<T>)>,
    pub style: Style,
    pub style_on_draw: Option<&'a StyleCallback<T>>,
    pub parent: usize,
    pub num_children: usize,
    pub last_child: Option<NonZeroUsize>,
    pub content: Content<'a, T>,
}

impl<'a, T> ArrayNode<'a, T> {
    // Note: Children are reversed
    pub(crate) fn child_ids(&self) -> std::ops::Range<usize> {
        if let Some(last_child) = self.last_child {
            last_child.get()..(last_child.get() + self.num_children)
        } else {
            0..0
        }
    }

    pub(crate) fn trigger(&self, event_type: On, state: &mut T, app: &mut App<T>) -> Stage {
        let mut stage = Stage::Idle;
        for (et, callback) in &self.callbacks {
            if *et == event_type {
                stage = stage.max((callback)(state, app));
            }
        }
        stage
    }
}

pub struct Node<'a, T> {
    key: Option<Key>,
    classes: Option<BumpVec<'a, &'static str>>,
    callbacks: Option<BumpVec<'a, (On, &'a EventCallback<T>)>>,
    style_default: Option<fn() -> Style>,
    style_on_draw: Option<&'a StyleCallback<T>>,
    size: usize,
    num_children: usize,
    prev_sibling: Option<&'a mut Node<'a, T>>,
    last_child: Option<&'a mut Node<'a, T>>,
    content: Content<'a, T>,
}

impl<'a, T> Node<'a, T> {
    pub fn new_in(al: &'a Alloc) -> Self {
        Self {
            key: None,
            classes: Some(BumpVec::new_in(&al.bump)),
            callbacks: Some(BumpVec::new_in(&al.bump)),
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
    pub fn event(mut self, event_type: On, callback: &'a EventCallback<T>) -> Self {
        if let Some(callbacks) = &mut self.callbacks {
            callbacks.push((event_type, callback));
        }
        self
    }

    /// Register a function that will provide an alternate default Style for this node
    pub fn style_default(mut self, func: fn() -> Style) -> Self {
        self.style_default = Some(func);
        self
    }

    /// Register a function to modify this node's style right before redrawing
    pub fn style_on_draw(mut self, func: &'a StyleCallback<T>) -> Self {
        self.style_on_draw = Some(func);
        self
    }

    /// Set the contents of a node
    pub fn content(mut self, content: Content<'a, T>) -> Self {
        self.content = content;
        self
    }

    /// Add a child node
    pub fn add_child(mut self, alloc: &'a Alloc, mut new_child: Self) -> Self {
        self.size += new_child.size;
        self.num_children += 1;

        if let Some(last_child) = self.last_child {
            new_child.size += last_child.size;
            new_child.prev_sibling = Some(last_child);
        }

        self.last_child = Some(alloc.bump.alloc(new_child));
        self
    }

    pub(crate) fn finish(mut self, al: &'a Alloc) -> Option<BumpVec<'a, ArrayNode<'a, T>>> {
        let mut tree: BumpVec<ArrayNode<T>> = BumpVec::with_capacity_in(self.size, &al.bump);
        let mut stack: BumpVec<(bool, usize, &mut Node<'a, T>)> = BumpVec::new_in(&al.bump);

        stack.push((false, 0, &mut self));
        while let Some((is_last_child, parent, curr_node)) = stack.pop() {
            let index = tree.len();
            if is_last_child {
                tree[parent].last_child = NonZeroUsize::new(index);
            }

            tree.push(ArrayNode {
                key: curr_node.key,
                classes: curr_node.classes.take()?,
                callbacks: curr_node.callbacks.take()?,
                style: curr_node.style_default.unwrap_or(Style::default)(),
                style_on_draw: curr_node.style_on_draw,
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
