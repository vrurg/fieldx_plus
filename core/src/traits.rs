pub trait Parent {
    type WeakSelf;

    fn __fxplus_myself_downgrade(&self) -> Self::WeakSelf;
}

pub trait Child {
    type RcParent;
    type WeakParent;
    type FXPParent;

    fn parent(&self) -> Self::RcParent;
    fn parent_downgrade(&self) -> Self::WeakParent;
    fn __fxplus_parent(parent: Self::WeakParent) -> Self::FXPParent;
}

pub trait Application: Parent {}

pub trait Agent {
    type RcApp;
    type WeakApp;
    type FXPApp;

    fn app(&self) -> Self::RcApp;
    fn app_downgrade(&self) -> Self::WeakApp;
    fn __fxplus_app(app: Self::WeakApp) -> Self::FXPApp;
}
