use std::fmt;
use std::num::NonZeroUsize;

use bumpalo::{collections::Vec as BumpVec, Bump};

use crate::app::*;
use crate::style::Style;

/// Macro for describing the structure and style of a UI
///
/// [] - Create and set classes on a new node
/// () - Set classes on interior instead of creating a new node
/// {} - Call methods on parent node
#[macro_export]
macro_rules! ui {
    ($alloc:ident; $($class:literal),* $(_)? => [ $($children:tt)* ]) => {
        ui!($alloc; TreeNode::new(&$alloc) $(.class(&$alloc, $class))*; $($children)* )
    };
    ($alloc:ident; $tree:expr; $($class:literal),* $(_)? => [ $($children:tt)* ] $($tail:tt)*) => {
        ui!($alloc; $tree.child(&$alloc, ui!($alloc; TreeNode::new(&$alloc) $(.class(&$alloc, $class))*; $($children)* )); $($tail)* )
    };
    ($alloc:ident; $tree:expr; $($class:literal),* $(_)? => ($widget:ident!( $($params:tt)* )) $($tail:tt)*) => {
        ui!($alloc; $tree.child(&$alloc, $widget!($alloc; $($params)*) $(.class(&$alloc, $class))* ); $($tail)* )
    };
    ($alloc:ident; $tree:expr; $($class:literal),* $(_)? => ( $function:expr ) $($tail:tt)*) => {
        ui!($alloc; $tree.child(&$alloc, $function $(&$alloc, .class(&$alloc, $class))* ); $($tail)* )
    };
    ($alloc:ident; $tree:expr; $($class:literal),* $(_)? => { $($builder:tt)* } $($tail:tt)*) => {
        ui!($alloc; $tree.child(&$alloc, TreeNode::new(&$alloc) $(.class(&$alloc, $class))* $($builder)* ); $($tail)* )
    };
    ($alloc:ident; $tree:expr; { $($body:tt)* } $($tail:tt)*) => {
        ui!($alloc; $tree $($body)*; $($tail)* )
    };

    // Control flow
    ($alloc:ident; $tree:expr; if let $v:pat = $e:tt $($tail:tt)*) => {
        ui!(@munch; @if_block; $alloc; $tree; tree; (if let $v = $e), $($tail)*)
    };
    ($alloc:ident; $tree:expr; if $e:tt $($tail:tt)*) => {
        ui!(@munch; @if_block; $alloc; $tree; tree; (if $e), $($tail)*)
    };
    ($alloc:ident; $tree:expr; for $v:pat in $e:tt $($tail:tt)*) => {
        ui!(@munch; @for_block; $alloc; $tree; tree; (for $v in $e), $($tail)*)
    };
    ($alloc:ident; $tree:expr; match $e:tt $($tail:tt)*) => {
        ui!(@munch; @match_block; $alloc; $tree; tree; (match $e), $($tail)*)
    };

    // If chains
    (@if_block; $alloc:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($body:tt)* } else if $e:tt $($tail:tt)*) => {
        ui!(@munch; $alloc; if_block; $tree; $temp; ($($prefix)* { $temp = ui!($alloc; $temp; $($body)* ); } else if $e), $($tail)* )
    };
    (@if_block; $alloc:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($one:tt)* } else { $($two:tt)* } $($tail:tt)*) => {
        ui!($alloc; { let mut $temp = $tree; $($prefix)* { $temp = ui!($alloc; $temp; $($one)* ); } else { $temp = ui!($alloc; $temp; $($two)* ); } $temp }; $($tail)* )
    };
    (@if_block; $alloc:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($body:tt)* } $($tail:tt)*) => {
        ui!($alloc; { let mut $temp = $tree; $($prefix)* { $temp = ui!($alloc; $temp; $($body)* ); } $temp }; $($tail)* )
    };

    // Match block
    (@match_block; $alloc:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($pattern:pat => { $($branch:tt)* } $(,)?)* } $($tail:tt)*) => {
        ui!($alloc; { let mut $temp = $tree; $temp = $($prefix)* { $($pattern => {ui!($alloc; $temp; $($branch)*)} )* }; $temp }; $($tail)* )
    };

    // For loop
    (@for_block; $alloc:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($body:tt)* } $($tail:tt)*) => {
        ui!($alloc; { let mut $temp = $tree; $($prefix)* { $temp = ui!($alloc; $temp; $($body)* ); } $temp }; $($tail)* )
    };

    // Prefix muncher
    (@munch; @$goto:ident; $alloc:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), { $($body:tt)* } $($tail:tt)*) => {
        ui!(@$goto; $alloc; $tree; $temp; ($($prefix)*), { $($body)* } $($tail)* )
    };
    (@munch; @$goto:ident; $alloc:ident; $tree:expr; $temp:ident; ($($prefix:tt)*), $first:tt $($tail:tt)*) => {
        ui!(@munch; @$goto; $alloc; $tree; $temp; ($($prefix)* $first), $($tail)* )
    };

    // Default case
    ($alloc:ident; $tree:expr; $($tail:tt)*) => {
        $tree $($tail)*
    };
}

pub type UI<'a, T> = TreeNode<'a, T>;

pub(crate) struct Callback<T>(fn(&mut T, &mut App) -> Redraw);

pub(crate) struct CallbackList<'a, T> {
    list: BumpVec<'a, (On, Callback<T>)>,
}

impl<'a, T> fmt::Debug for CallbackList<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CallbackList[{}]", self.list.len())
    }
}

impl<'a, T> CallbackList<'a, T> {
    fn new(alloc: &'a Bump) -> Self {
        Self {
            list: BumpVec::new_in(&alloc),
        }
    }

    fn add(&mut self, event_type: On, callback: fn(&mut T, &mut App) -> Redraw) {
        self.list.push((event_type, Callback(callback)));
    }

    pub(crate) fn trigger(&self, event_type: On, store: &mut T, app: &mut App) -> Redraw {
        let mut redraw = Redraw::No;
        for (et, callback) in &self.list {
            if *et == event_type {
                let value = callback.0(store, app);
                if value == Redraw::Yes {
                    redraw = Redraw::Yes;
                }
            }
        }
        redraw
    }
}

pub enum Content<'a, T> {
    None,
    Text(&'a str),
    DynamicText(fn(&'a T) -> &str),
}

impl<'a, T> Default for Content<'a, T> {
    fn default() -> Self {
        Self::None
    }
}

pub(crate) struct ArrayNode<'a, T> {
    pub id: Option<&'a str>,
    pub classes: BumpVec<'a, &'a str>,
    pub callbacks: CallbackList<'a, T>,
    pub style: Style,
    pub parent: usize,
    pub num_children: usize,
    pub last_child: Option<NonZeroUsize>,
    pub content: Content<'a, T>,
}

impl<'a, T> ArrayNode<'a, T> {
    pub fn child_ids(&self) -> std::ops::Range<usize> {
        if let Some(last_child) = self.last_child {
            last_child.get()..(last_child.get() + self.num_children)
        } else {
            0..0
        }
    }
}

pub struct TreeNode<'a, T> {
    id: Option<&'a str>,
    classes: Option<BumpVec<'a, &'a str>>,
    callbacks: Option<CallbackList<'a, T>>,
    style_default: Option<&'a dyn Fn() -> Style>,
    size: usize,
    num_children: usize,
    prev_sibling: Option<&'a mut TreeNode<'a, T>>,
    last_child: Option<&'a mut TreeNode<'a, T>>,
    content: Content<'a, T>,
}

impl<'a, T> TreeNode<'a, T> {
    pub fn new(alloc: &'a Bump) -> Self {
        Self {
            id: None,
            classes: Some(BumpVec::new_in(&alloc)),
            callbacks: Some(CallbackList::new(&alloc)),
            style_default: None,
            size: 1,
            num_children: 0,
            prev_sibling: None,
            last_child: None,
            content: Content::None,
        }
    }
}

impl<'a, T> TreeNode<'a, T> {
    /// Set the id
    pub fn id(mut self, alloc: &'a Bump, id: &str) -> Self {
        self.id = Some(alloc.alloc_str(id));
        self
    }

    /// Add a CSS class
    pub fn class(mut self, alloc: &'a Bump, class: &'a str) -> Self {
        if let Some(classes) = &mut self.classes {
            classes.push(alloc.alloc(class.clone()));
        }
        self
    }

    /// Register an event listener
    pub fn event(mut self, event_type: On, callback: fn(&mut T, &mut App) -> Redraw) -> Self {
        if let Some(callbacks) = &mut self.callbacks {
            callbacks.add(event_type, callback);
        }
        self
    }

    /// Register a function that will provide an alternate default Style for this node
    pub fn style_default(mut self, alloc: &'a Bump, func: &'a dyn Fn() -> Style) -> Self {
        self.style_default = Some(alloc.alloc(func));
        self
    }

    /// Set the contents of a node
    pub fn content(mut self, content: Content<'a, T>) -> Self {
        self.content = content;
        self
    }

    /// Add a child node
    pub fn child(mut self, alloc: &'a Bump, mut new_child: Self) -> Self {
        self.size += new_child.size;
        self.num_children += 1;

        if let Some(last_child) = self.last_child {
            new_child.size += last_child.size;
            new_child.prev_sibling = Some(last_child);
        }

        self.last_child = Some(alloc.alloc(new_child));
        self
    }

    pub(crate) fn finish(mut self, alloc: &'a Bump) -> Option<BumpVec<'a, ArrayNode<'a, T>>> {
        let mut tree: BumpVec<ArrayNode<T>> = BumpVec::with_capacity_in(self.size, &alloc);
        let mut stack: BumpVec<(bool, usize, &mut TreeNode<'a, T>)> = BumpVec::new_in(&alloc);

        stack.push((false, 0, &mut self));
        while let Some((is_last_child, parent, curr_node)) = stack.pop() {
            let index = tree.len();
            if is_last_child {
                tree[parent].last_child = NonZeroUsize::new(index);
            }

            let style = if let Some(style_default) = curr_node.style_default {
                style_default()
            } else {
                Style::default()
            };

            tree.push(ArrayNode {
                id: curr_node.id,
                classes: curr_node.classes.take()?,
                callbacks: curr_node.callbacks.take()?,
                style,
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
