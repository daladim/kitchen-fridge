//! This crate provides a way to manage CalDAV data.
//!
//! Its initial implementation only supported TODO events, so that it could fetch and update a CalDAV-hosted todo-list...just like [sticky notes on a kitchen fridge](https://www.google.com/search?q=kitchen+fridge+todo+list&tbm=isch) would. \
//! Supporting other items (and especially regular CalDAV calendar events) should be fairly trivial, as it should boil down to adding little logic in iCal files parsing, but any help is appreciated :-)
//!
//! ## Possible uses
//!
//! It provides a CalDAV client in the [`client`] module, that can be used as a stand-alone module.
//!
//! Because the connection to the server may be slow, this crate also provides a local cache for CalDAV data in the [`cache`] module.
//! This way, user-frendly apps are able to quicky display cached data on startup.
//!
//! These two "data sources" (actual client and local cache) can be used together in a [`CalDavProvider`](CalDavProvider). \
//! A `CalDavProvider` abstracts these two sources by merging them together into one virtual source. \
//! It also handles synchronisation between the local cache and the server.
//!
//! Note that many methods are defined in common traits (see [`crate::traits`]).
//!
//! ## Examples
//!
//! See example usage in the `examples/` folder, that you can run using `cargo run --example <example-name>`

pub mod traits;

pub mod calendar;
pub mod item;
pub use item::Item;
pub mod task;
pub use task::Task;
pub mod event;
pub use event::Event;
pub mod provider;
pub mod mock_behaviour;

pub mod client;
pub use client::Client;
pub mod cache;
pub use cache::Cache;
pub mod ical;

pub mod settings;
pub mod utils;
pub mod resource;

/// Unless you want another kind of Provider to write integration tests, you'll probably want this kind of Provider. \
/// See alse the [`Provider` documentation](crate::provider::Provider)
pub type CalDavProvider = provider::Provider<cache::Cache, calendar::cached_calendar::CachedCalendar, Client, calendar::remote_calendar::RemoteCalendar>;
