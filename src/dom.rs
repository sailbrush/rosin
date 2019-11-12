use crate::app::*;

#[macro_export]
macro_rules! dom {
    ($(#$id:ident)? $(,)? $($class:ident),* [ $($block:tt)* ]) => {
        dom!( Dom::div() $(.id(stringify!(#$id)))* $(.class(stringify!($class)))*, $($block)* )
    };
    ($node:expr, [@ $($builder:tt)* ] $($tail:tt)*) => {
        dom!( $node$($builder)*, $($tail)* )
    };
    ($node:expr, $(#$id:ident)? $(,)? $($class:ident),* [^ $block:expr ] $($tail:tt)*) => {
        dom!( $node.child( $block $(.id(stringify!(#$id)))* $(.class(stringify!($class)))* ), $($tail)* )
    };
    ($node:expr, $(#$id:ident)? $(,)? $($class:ident),* [ $($block:tt)* ] $($tail:tt)*) => {
        dom!( $node.child(dom!( Dom::div() $(.id(stringify!(#$id)))* $(.class(stringify!($class)))*, $($block)* )), $($tail)* )
    };
    ($node:expr, $e:expr; $($tail:tt)*) => {
        dom!( $node.child($e), $($tail)* )
    };
    ($node:expr, $($tail:tt)*) => {
        $node $($tail)*
    };
}

pub type NodeId = usize;

#[derive(Debug)]
pub enum NodeType {
    Div,
}

#[derive(Debug)]
pub struct Node<T> {
    pub(crate) node_type: NodeType,
    pub(crate) children_total: usize,

    pub(crate) parent: Option<NodeId>,
    pub(crate) prev_sibling: Option<NodeId>,
    pub(crate) next_sibling: Option<NodeId>,
    pub(crate) first_child: Option<NodeId>,

    pub(crate) label: Option<String>,
    pub(crate) css_id: Option<&'static str>,
    pub(crate) css_classes: Vec<&'static str>,
    pub(crate) callbacks: CallbackList<T>,
}

impl<T> Node<T> {
    pub fn new(new_type: NodeType) -> Self {
        Node {
            node_type: new_type,
            children_total: 0,

            parent: None,
            prev_sibling: None,
            next_sibling: None,
            first_child: None,

            label: None,
            css_id: None,
            css_classes: Vec::new(),
            callbacks: CallbackList::new(),
        }
    }
}

#[derive(Debug)]
pub struct Dom<T> {
    pub(crate) arena: Vec<Node<T>>,
}

impl<T> Dom<T> {
    pub fn new(node_type: NodeType) -> Self {
        let mut dom = Vec::new();
        dom.push(Node::new(node_type));
        Dom { arena: dom }
    }

    pub fn div() -> Self {
        Self::new(NodeType::Div)
    }

    pub fn id(mut self, id: &'static str) -> Self {
        self.arena[0].css_id = Some(id);
        self
    }

    pub fn class(mut self, class: &'static str) -> Self {
        self.arena[0].css_classes.push(class);
        self
    }

    pub fn label<S: ToString>(mut self, label: S) -> Self {
        self.arena[0].label = Some(label.to_string());
        self
    }

    pub fn event(
        mut self,
        event_type: On,
        callback: fn(&mut T, app: &mut App<T>) -> Redraw,
    ) -> Self {
        self.arena[0].callbacks.insert(event_type, callback);
        self
    }

    pub fn child(mut self, mut child: Self) -> Self {
        let shift_amt = self.arena.len();

        // Update and move children into parent arena
        for node in child.arena.iter_mut() {
            if let Some(parent) = node.parent.as_mut() {
                *parent += shift_amt;
            }

            if let Some(prev_sibling) = node.prev_sibling.as_mut() {
                *prev_sibling += shift_amt;
            }

            if let Some(next_sibling) = node.next_sibling.as_mut() {
                *next_sibling += shift_amt;
            }

            if let Some(first_child) = node.first_child.as_mut() {
                *first_child += shift_amt;
            }
        }
        self.arena.append(&mut child.arena);

        // Fix references to/from new child
        self.arena[shift_amt].parent = Some(0);
        self.arena[0].children_total += 1;
        if let Some(first_child_id) = self.arena[0].first_child {
            if let Some(last_child_id) = self.arena[first_child_id].prev_sibling {
                self.arena[first_child_id].prev_sibling = Some(shift_amt);
                self.arena[last_child_id].next_sibling = Some(shift_amt);
                self.arena[shift_amt].prev_sibling = Some(last_child_id);
                self.arena[shift_amt].next_sibling = Some(first_child_id);
            } else {
                self.arena[first_child_id].prev_sibling = Some(shift_amt);
                self.arena[first_child_id].next_sibling = Some(shift_amt);
                self.arena[shift_amt].prev_sibling = Some(first_child_id);
                self.arena[shift_amt].next_sibling = Some(first_child_id);
            }
        } else {
            self.arena[0].first_child = Some(shift_amt);
            self.arena[shift_amt].prev_sibling = Some(shift_amt);
            self.arena[shift_amt].next_sibling = Some(shift_amt);
        }
        self
    }

    pub fn get_children(&self, id: NodeId) -> Vec<NodeId> {
        if let Some(first_child) = self.arena[id].first_child {
            let mut current_node = first_child;
            let mut child_ids: Vec<NodeId> = Vec::with_capacity(self.arena[id].children_total);
            for _ in 0..self.arena[id].children_total {
                child_ids.push(current_node);
                current_node = self.arena[current_node]
                    .next_sibling
                    .expect("[Rosin] Malformed Dom");
            }
            child_ids
        } else {
            Vec::with_capacity(0)
        }
    }
}
