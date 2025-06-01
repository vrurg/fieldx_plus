/// This trait is used to declare parent structs in parent-child relationships.
pub trait Parent {
    /// Weak ref-count type for the parent to hold a reference to itself.
    type WeakSelf;

    fn __fxplus_myself_downgrade(&self) -> Self::WeakSelf;
}

/// This trait is used to declare child structs in parent-child relationships.
pub trait Child {
    /// Type of strong reference to the parent.
    type RcParent;
    /// Type of weak reference to the parent.
    type WeakParent;
    /// For use of the [`child_build!`](crate::child_build!) and [`child_builder!`](crate::child_builder) macros.
    type FXPParent;

    /// Return a strong reference to the parent.
    fn parent(&self) -> Self::RcParent;
    /// Return a weak reference to the parent.
    fn parent_downgrade(&self) -> Self::WeakParent;
    fn __fxplus_parent(parent: Self::WeakParent) -> Self::FXPParent;
}

/// This trait is used to declare application structs. For now it is just a marker trait,
/// but it may be extended in the future to include application-specific methods or properties.
pub trait Application: Parent {}

/// This trait is used to declare agents that can access the application. Technically, it is identical to the
/// [`Child`](crate::Child) trait, but it is used to distinguish _agents_ from _children_ because an agent
/// can also be a child of a parent that is different from the application itself. Besides, specialized method
/// names improve code readability and intent clarity.
pub trait Agent {
    /// Type of strong reference to the application.
    type RcApp;
    /// Type of weak reference to the application.
    type WeakApp;
    /// For use of the [`agent_build!`](crate::agent_build!) and [`agent_builder!`](crate::agent_builder) macros.
    type FXPApp;

    /// Return a strong reference to the application.
    fn app(&self) -> Self::RcApp;
    /// Return a weak reference to the application.
    fn app_downgrade(&self) -> Self::WeakApp;
    fn __fxplus_app(app: Self::WeakApp) -> Self::FXPApp;
}
