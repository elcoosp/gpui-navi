//! Navi router devtools — framework-agnostic panel built on quoin's
//! `Node` UI contract. The host application renders the model through a
//! quoin adapter (the example app uses `quoin-ui-gpui`).

pub mod devtools;
pub mod timeline;

pub use devtools::{
    DevtoolsModel, DevtoolsTab, EventColors, EventDetail, EventDisplay, RouterEventType,
    SharedModel, ids,
};
