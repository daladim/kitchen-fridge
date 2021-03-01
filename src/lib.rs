//! This crate provides a way to manage CalDAV data.
//!
//! It provides a CalDAV client in the [`client`] module, that can be used as a stand-alone module.
//!
//! Because the connection to the server may be slow, and a user-frendly app may want to quicky display cached data on startup, this crate also provides a local cache for CalDAV data in the [`cache`] module.
//!
//! These two "data sources" (actual client and local cache) can be used together in a [`Provider`](provider::Provider). \
//! A `Provider` abstracts these two sources by merging them together into one virtual source. \
//! It also handles synchronisation between the local cache and the server.

pub mod traits;

pub mod calendar;
pub use calendar::cached_calendar::CachedCalendar;
mod item;
pub use item::Item;
mod task;
pub use task::Task;
mod event;
pub use event::Event;
pub mod provider;
pub use provider::Provider;

pub mod client;
pub mod cache;

pub mod settings;
pub mod utils;
