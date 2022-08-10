use downcast_rs::{Downcast, impl_downcast};
use std::{fmt::Debug, any::TypeId};
use crate::Renderer;
use crate::common::{ErasedSet, TypeKey};
use crate::common::sourcemap::SourcePos;
use crate::parser::renderer::HTMLRenderer;

/// Single node in the CommonMark AST.
#[derive(Debug)]
#[readonly::make]
pub struct Node {
    /// Array of child nodes.
    pub children: Vec<Node>,

    /// Source mapping info.
    pub srcmap: Option<SourcePos>,

    /// Custom data specific to this token.
    pub env: ErasedSet,

    /// Additional attributes to be added to resulting html.
    pub attrs: Vec<(&'static str, String)>,

    /// Type name, used for debugging.
    #[readonly]
    pub node_type: TypeKey,

    /// Storage for arbitrary token-specific data.
    #[readonly]
    pub node_value: Box<dyn NodeValue>,
}

impl Node {
    /// Create a new [Node](Node) with a custom value.
    pub fn new<T: NodeValue>(value: T) -> Self {
        Self {
            children:   Vec::new(),
            srcmap:     None,
            attrs:      Vec::new(),
            env:        ErasedSet::new(),
            node_type:  TypeKey::of::<T>(),
            node_value: Box::new(value),
        }
    }

    /// Return std::any::type_name() of node value.
    pub fn name(&self) -> &'static str {
        self.node_type.name
    }

    /// Check that this node value is of given type.
    pub fn is<T: NodeValue>(&self) -> bool {
        self.node_type.id == TypeId::of::<T>()
    }

    /// Downcast node value to specific type.
    pub fn cast<T: NodeValue>(&self) -> Option<&T> {
        if self.node_type.id == TypeId::of::<T>() {
            Some(self.node_value.downcast_ref::<T>().unwrap())
            // performance note: `node_type.id` improves walk speed by a LOT by removing indirection
            // (~5% of overall program speed), so having type id duplicated in Node is very beneficial;
            // we can also remove extra check with downcast_unchecked, but it doesn't do much
            //Some(unsafe { &*(&*self.node_value as *const dyn NodeValue as *const T) })
        } else {
            None
        }
    }

    /// Downcast node value to specific type.
    pub fn cast_mut<T: NodeValue>(&mut self) -> Option<&mut T> {
        if self.node_type.id == TypeId::of::<T>() {
            Some(self.node_value.downcast_mut::<T>().unwrap())
            // performance note: see above
            //Some(unsafe { &mut *(&mut *self.node_value as *mut dyn NodeValue as *mut T) })
        } else {
            None
        }
    }

    /// Render this node to HTML.
    pub fn render(&self) -> String {
        let mut fmt = HTMLRenderer::<false>::new();
        fmt.render(self);
        fmt.into()
    }

    /// Render this node to XHTML, it adds slash to self-closing tags like this: `<img />`.
    ///
    /// This mode exists for compatibility with CommonMark tests.
    pub fn xrender(&self) -> String {
        let mut fmt = HTMLRenderer::<true>::new();
        fmt.render(self);
        fmt.into()
    }

    /// Replace custom value with another value (this is roughly equivalent
    /// to replacing the entire node and copying children and sourcemaps).
    pub fn replace<T: NodeValue>(&mut self, value: T) {
        self.node_type  = TypeKey::of::<T>();
        self.node_value = Box::new(value);
    }

    /// Execute function `f` recursively on every member of AST tree
    /// (using preorder deep-first search).
    pub fn walk(&self, mut f: impl FnMut(&Node, u32)) {
        // performance note: this is faster than emulating recursion using vec stack
        fn walk_recursive(node: &Node, depth: u32, f: &mut impl FnMut(&Node, u32)) {
            f(node, depth);
            for n in node.children.iter() {
                walk_recursive(n, depth + 1, f);
            }
        }

        walk_recursive(self, 0, &mut f);
    }

    /// Execute function `f` recursively on every member of AST tree
    /// (using preorder deep-first search).
    pub fn walk_mut(&mut self, mut f: impl FnMut(&mut Node, u32)) {
        // performance note: this is faster than emulating recursion using vec stack
        fn walk_recursive(node: &mut Node, depth: u32, f: &mut impl FnMut(&mut Node, u32)) {
            f(node, depth);
            for n in node.children.iter_mut() {
                walk_recursive(n, depth + 1, f);
            }
        }

        walk_recursive(self, 0, &mut f);
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct NodeEmpty;
impl NodeValue for NodeEmpty {}

impl Default for Node {
    /// Create empty Node. Empty node should only be used as placeholder for functions like
    /// std::mem::take, and it cannot be rendered.
    fn default() -> Self {
        Node::new(NodeEmpty)
    }
}

/// Contents of the specific AST node.
pub trait NodeValue : Debug + Downcast {
    /// Output HTML corresponding to this node using Renderer API.
    ///
    /// Example implementation looks like this:
    /// ```rust
    /// # const IGNORE : &str = stringify! {
    /// fn render(&self, node: &Node, fmt: &mut dyn Renderer) {
    ///    fmt.open("div", &[]);
    ///    fmt.contents(&node.children);
    ///    fmt.close("div");
    ///    fmt.cr();
    /// }
    /// # };
    /// ```
    fn render(&self, node: &Node, fmt: &mut dyn Renderer) {
        let _ = fmt;
        unimplemented!("{} doesn't implement render", node.name());
    }
}

impl_downcast!(NodeValue);
