use crate::alloc::Alloc;
use crate::geometry::Size;
use crate::prelude::*;
use crate::stylesheet::Stylesheet;

use std::collections::HashMap;
use std::num::NonZeroUsize;

use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump;

/// Macro for describing the structure and style of a UI.
///
/// [ ] - Create and set classes on a new node.
/// ( ) - Set classes on interior instead of creating a new node.
/// { } - Call methods on parent node.
#[macro_export]
macro_rules! ui {
    ($($classes:literal)? [ $($children:tt)* ]) => {
        ui!(View::default() $(.add_classes($classes))*; $($children)* )
    };
    ($sheet:expr, $($classes:literal)? [ $($children:tt)* ]) => {
        ui!(View::default().use_style_sheet(Some($sheet)) $(.add_classes($classes))*; $($children)* )
    };
    ($tree:expr; $($classes:literal)? [ $($children:tt)* ] $($tail:tt)*) => {
        ui!($tree.add_child(ui!(View::default() $(.add_classes($classes))*; $($children)* )); $($tail)* )
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
    pub key: Option<Key>,
    pub classes: BumpVec<'static, &'static str>,
    pub event_callbacks: BumpVec<'static, (On, &'static mut dyn EventCallback<S, H>)>,
    pub style_sheet: Option<Stylesheet>,
    pub style_callback: Option<&'static mut dyn StyleCallback<S>>,
    pub layout_callback: Option<&'static mut dyn LayoutCallback<S>>,
    pub draw_callback: Option<&'static mut dyn DrawCallback<S>>,
    pub _draw_cache_enable: bool, // TODO
    pub parent: usize,
    pub num_children: usize,
    pub last_child: Option<NonZeroUsize>,
}

impl<S, H> Drop for ArrayNode<S, H> {
    fn drop(&mut self) {
        for cb in &mut self.event_callbacks {
            unsafe {
                std::ptr::drop_in_place(cb.1);
            }
        }
        if let Some(cb) = &mut self.style_callback {
            unsafe {
                std::ptr::drop_in_place(*cb);
            }
        }
        if let Some(cb) = &mut self.layout_callback {
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

impl<S, H> std::fmt::Debug for ArrayNode<S, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArrayNode")
            .field("key", &self.key)
            .field("classes", &self.classes)
            .field("event_callbacks", &self.event_callbacks.len())
            .field("style_sheet", &self.style_sheet)
            .field("style_callback", &self.style_callback.is_some())
            .field("layout_callback", &self.layout_callback.is_some())
            .field("draw_callback", &self.draw_callback.is_some())
            .field("_draw_cache_enable", &self._draw_cache_enable)
            .field("parent", &self.parent)
            .field("num_children", &self.num_children)
            .field("last_child", &self.last_child)
            .finish()
    }
}

impl<S, H> ArrayNode<S, H> {
    // Note: Children are reversed
    pub(crate) fn child_ids(&self) -> Option<std::ops::Range<usize>> {
        self.last_child
            .map(|last_child| last_child.get()..(last_child.get() + self.num_children))
    }

    pub fn run_callbacks(&mut self, event_type: On, state: &mut S, ctx: &mut EventCtx<S, H>) -> Phase {
        let mut phase = Phase::Idle;
        for (et, callback) in &mut self.event_callbacks {
            if *et == event_type {
                phase.update((callback)(state, ctx).unwrap_or(Phase::Idle));
            }
        }
        phase
    }

    pub fn has_callback(&self, event_type: On) -> bool {
        for (et, _) in &self.event_callbacks {
            if *et == event_type {
                return true;
            }
        }
        false
    }
}

/// A node in the view tree. Panics if created outside of a `ViewCallback`.
#[allow(clippy::type_complexity)]
pub struct View<S: 'static, H: 'static> {
    key: Option<Key>,
    classes: Option<BumpVec<'static, &'static str>>,
    style_sheet: Option<Stylesheet>,
    event_callbacks: Option<BumpVec<'static, (On, &'static mut dyn EventCallback<S, H>)>>,
    style_callback: Option<&'static mut dyn StyleCallback<S>>,
    layout_callback: Option<&'static mut dyn LayoutCallback<S>>,
    draw_callback: Option<&'static mut dyn DrawCallback<S>>,
    draw_cache_enable: bool,
    tree_size: usize,
    num_children: usize,
    prev_sibling: Option<&'static mut View<S, H>>,
    last_child: Option<&'static mut View<S, H>>,
}

impl<S, H> Default for View<S, H> {
    fn default() -> Self {
        let alloc = Alloc::get_thread_local_alloc().unwrap();
        alloc.increment_counter();

        Self {
            key: None,
            classes: Some(alloc.vec()),
            style_sheet: None,
            event_callbacks: Some(alloc.vec()),
            style_callback: None,
            layout_callback: None,
            draw_callback: None,
            draw_cache_enable: false,
            tree_size: 1,
            num_children: 0,
            prev_sibling: None,
            last_child: None,
        }
    }
}

impl<S, H> View<S, H> {
    /// Set a key on a node, providing a stable identity between rebuilds.
    pub fn key(mut self, key: Key) -> Self {
        self.key = Some(key);
        self
    }

    /// Register an event callback.
    pub fn event(mut self, event_type: On, callback: impl Fn(&mut S, &mut EventCtx<S, H>) -> Option<Phase> + 'static) -> Self {
        if let Some(callbacks) = &mut self.event_callbacks {
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
    pub fn on_layout(mut self, func: impl Fn(&S, Size) + 'static) -> Self {
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

        self.tree_size += new_child.tree_size;
        self.num_children += 1;

        if let Some(last_child) = self.last_child {
            new_child.tree_size += last_child.tree_size;
            new_child.prev_sibling = Some(last_child);
        }

        self.last_child = Some(alloc.alloc(new_child));
        self
    }

    pub fn use_style_sheet(mut self, style_sheet: Option<Stylesheet>) -> Self {
        self.style_sheet = style_sheet;
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

    pub(crate) fn finish(mut self, temp: &Bump, key_map: &mut HashMap<Key, usize>) -> Option<BumpVec<'static, ArrayNode<S, H>>> {
        let alloc = Alloc::get_thread_local_alloc().unwrap();

        let mut tree: BumpVec<ArrayNode<S, H>> = alloc.vec_capacity(self.tree_size);
        let mut stack: BumpVec<(bool, usize, &mut View<S, H>)> = BumpVec::new_in(temp);

        // Root's parent is set to usize::MAX, since it would be impossible for the node at the end of the largest possible array to have children
        stack.push((false, usize::MAX, &mut self));
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
                style_sheet: curr_node.style_sheet.take(),
                event_callbacks: curr_node.event_callbacks.take()?,
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
