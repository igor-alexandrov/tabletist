//! An object that calls a closure when a menu item is chosen: what a menu
//! item of ours points at.

// Defining an Objective-C class is a message the compiler cannot check.
#![allow(unsafe_code)]

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};

pub(crate) struct Ivars {
    chosen: Box<dyn Fn()>,
}

define_class!(
    // SAFETY: NSObject has no rules for its subclasses, and `MenuTarget`
    // does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    pub(crate) struct MenuTarget;

    impl MenuTarget {
        // SAFETY: an action takes its sender (an object or nil) and returns
        // nothing.
        #[unsafe(method(menuItemChosen:))]
        fn menu_item_chosen(&self, _sender: Option<&AnyObject>) {
            (self.ivars().chosen)();
        }
    }
);

impl MenuTarget {
    pub(crate) fn new(mtm: MainThreadMarker, chosen: Box<dyn Fn()>) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(Ivars { chosen });
        // SAFETY: NSObject's `init` takes nothing and returns the object.
        unsafe { msg_send![super(this), init] }
    }
}
