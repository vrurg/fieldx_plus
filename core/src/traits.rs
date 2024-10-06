pub trait Parent {
    type WeakSelf;
    fn __fxplus_myself_downgrade(&self) -> Self::WeakSelf;
}

pub trait Child {
    type RcParent;
    type WeakParent;

    fn parent(&self) -> Self::RcParent;
    fn parent_downgrade(&self) -> Self::WeakParent;
}

pub trait Application: Parent {}

pub trait Agent {
    type RcApp;
    type WeakApp;

    fn app(&self) -> Self::RcApp;
    fn app_downgrade(&self) -> Self::WeakApp;
}
