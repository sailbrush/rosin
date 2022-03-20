use crate::alloc::Alloc;
use crate::geometry::Size;
use crate::prelude::*;

use std::collections::HashMap;
use std::num::NonZeroUsize;

use bumpalo::collections::Vec as BumpVec;

/// Macro for describing the structure and style of a UI.
///
/// [ ] - Create and set classes on a new node.
/// ( ) - Set classes on interior instead of creating a new node.
/// { } - Call methods on parent node.
#[macro_export]
macro_rules! ui {
    ($($classes:literal)? [ $($children:tt)* ]) => {
        ui!(Node::default() $(.add_classes($classes))*; $($children)* )
    };
    ($sheet:expr, $($classes:literal)? [ $($children:tt)* ]) => {
        ui!(Node::default().use_style_sheet($sheet) $(.add_classes($classes))*; $($children)* )
    };
    ($tree:expr; $($classes:literal)? [ $($children:tt)* ] $($tail:tt)*) => {
        ui!($tree.add_child(ui!(Node::default() $(.add_classes($classes))*; $($children)* )); $($tail)* )
    };
    ($tree:expr; $($classes:literal)? ( $($child:tt)* ) $($tail:tt)*) => {
        ui!($tree.add_child($($child)* $(.add_classes($classes))* ); $($tail)* )
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

pub(crate) struct ArrayNode<S: 'static, H: 'static> {
    pub key: Option<Key>, // TODO
    pub classes: BumpVec<'static, &'static str>,
    pub callbacks: BumpVec<'static, (On, &'static mut dyn EventCallback<S, H>)>,
    pub style_sheet: Option<StyleSheetId>,
    pub style_callback: Option<&'static mut dyn StyleCallback<S>>,
    pub layout_callback: Option<&'static mut dyn LayoutCallback<S>>,
    pub draw_callback: Option<&'static mut dyn DrawCallback<S>>,
    pub _draw_cache_enable: bool, // TODO
    pub parent: usize,
    pub num_children: usize,
    pub last_child: Option<NonZeroUsize>,

    // The only field that should ever be mutated after creation
    pub style: Style,
}

impl<S, H> Drop for ArrayNode<S, H> {
    fn drop(&mut self) {
        for cb in &mut self.callbacks {
            unsafe {
                std::ptr::drop_in_place(cb.1);
            }
        }
        if let Some(cb) = &mut self.style_callback {
            unsafe {
                std::ptr::drop_in_place(*cb);
            }
        }
        if let Some(cb) = &mut self.draw_callback {
            unsafe {
                std::ptr::drop_in_place(*cb);
            }
        }
    }
}

impl<S, H> ArrayNode<S, H> {
    // Note: Children are reversed
    pub(crate) fn child_ids(&self) -> std::ops::Range<usize> {
        if let Some(last_child) = self.last_child {
            last_child.get()..(last_child.get() + self.num_children)
        } else {
            0..0
        }
    }

    pub fn run_callbacks(&mut self, event_type: On, state: &mut S, ctx: &mut EventCtx<S, H>) -> Phase {
        let mut phase = Phase::Idle;
        for (et, callback) in &mut self.callbacks {
            if *et == event_type {
                phase.update((callback)(state, ctx));
            }
        }
        phase
    }

    pub fn has_callback(&self, event_type: On) -> bool {
        for (et, _) in &self.callbacks {
            if *et == event_type {
                return true;
            }
        }
        return false;
    }
}

/// A node in the view tree. Panics if created outside of a `ViewCallback`.
pub struct Node<S: 'static, H: 'static> {
    key: Option<Key>,
    classes: Option<BumpVec<'static, &'static str>>,
    callbacks: Option<BumpVec<'static, (On, &'static mut dyn EventCallback<S, H>)>>,
    style_sheet: Option<StyleSheetId>,
    style_callback: Option<&'static mut dyn StyleCallback<S>>,
    layout_callback: Option<&'static mut dyn LayoutCallback<S>>,
    draw_callback: Option<&'static mut dyn DrawCallback<S>>,
    draw_cache_enable: bool,
    size: usize,
    num_children: usize,
    prev_sibling: Option<&'static mut Node<S, H>>,
    last_child: Option<&'static mut Node<S, H>>,
}

impl<S, H> Default for Node<S, H> {
    fn default() -> Self {
        let alloc = Alloc::get_thread_local_alloc().unwrap();
        alloc.increment_counter();

        Self {
            key: None,
            classes: Some(alloc.vec()),
            callbacks: Some(alloc.vec()),
            style_sheet: None,
            style_callback: None,
            layout_callback: None,
            draw_callback: None,
            draw_cache_enable: false,
            size: 1,
            num_children: 0,
            prev_sibling: None,
            last_child: None,
        }
    }
}

impl<S, H> Node<S, H> {
    /// Set a key on a node, providing a stable identity between rebuilds.
    pub fn key(mut self, key: Key) -> Self {
        self.key = Some(key);
        self
    }

    /// Register an event callback.
    pub fn event(mut self, event_type: On, callback: impl Fn(&mut S, &mut EventCtx<S, H>) -> Phase + 'static) -> Self {
        if let Some(callbacks) = &mut self.callbacks {
            let alloc = Alloc::get_thread_local_alloc().unwrap();
            callbacks.push((event_type, alloc.alloc(callback)));
        }
        self
    }

    /// Register a function to modify this node's style before drawing.
    pub fn on_style(mut self, func: impl Fn(&S, &mut Style) + 'static) -> Self {
        let alloc = Alloc::get_thread_local_alloc().unwrap();
        self.style_callback = Some(alloc.alloc(func));
        self
    }

    /// Register a funciton to layout the contents of this node.
    pub fn on_layout(mut self, func: impl Fn(&mut S, Size) + 'static) -> Self {
        let alloc = Alloc::get_thread_local_alloc().unwrap();
        self.layout_callback = Some(alloc.alloc(func));
        self
    }

    /// Register a function to draw the contents of this node.
    pub fn on_draw(mut self, enable_cache: bool, func: impl Fn(&S, &mut DrawCtx) + 'static) -> Self {
        let alloc = Alloc::get_thread_local_alloc().unwrap();
        self.draw_callback = Some(alloc.alloc(func));
        self.draw_cache_enable = enable_cache;
        self
    }

    /// Add a child node.
    pub fn add_child(mut self, mut new_child: Self) -> Self {
        let alloc = Alloc::get_thread_local_alloc().unwrap();

        self.size += new_child.size;
        self.num_children += 1;

        if let Some(last_child) = self.last_child {
            new_child.size += last_child.size;
            new_child.prev_sibling = Some(last_child);
        }

        self.last_child = Some(alloc.alloc(new_child));
        self
    }

    pub fn use_style_sheet(mut self, id: StyleSheetId) -> Self {
        self.style_sheet = Some(id);
        self
    }

    pub fn add_classes(mut self, classes: &'static str) -> Self {
        if let Some(class_vec) = &mut self.classes {
            for class in classes.split_whitespace() {
                class_vec.push(class);
            }
        }
        self
    }

    pub(crate) fn finish(mut self, key_map: &mut HashMap<Key, usize>) -> Option<BumpVec<'static, ArrayNode<S, H>>> {
        let alloc = Alloc::get_thread_local_alloc().unwrap();

        let mut tree: BumpVec<ArrayNode<S, H>> = alloc.vec_capacity(self.size);
        let mut stack: BumpVec<(bool, usize, &mut Node<S, H>)> = alloc.vec();

        stack.push((false, 0, &mut self));
        while let Some((is_last_child, parent, curr_node)) = stack.pop() {
            let index = tree.len();
            if is_last_child {
                tree[parent].last_child = NonZeroUsize::new(index);
            }

            if let Some(key) = curr_node.key {
                key_map.insert(key, tree.len());
            }

            tree.push(ArrayNode {
                key: curr_node.key,
                classes: curr_node.classes.take()?,
                callbacks: curr_node.callbacks.take()?,
                style_sheet: curr_node.style_sheet.take(),
                style: Style::default(),
                style_callback: curr_node.style_callback.take(),
                layout_callback: curr_node.layout_callback.take(),
                draw_callback: curr_node.draw_callback.take(),
                _draw_cache_enable: curr_node.draw_cache_enable,
                parent,
                num_children: curr_node.num_children,
                last_child: None,
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
