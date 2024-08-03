//! This crate provides a CalDAV client library. \
//! CalDAV is described as "Calendaring Extensions to WebDAV" in [RFC 4791](https://datatracker.ietf.org/doc/html/rfc4791) and [RFC 7986](https://datatracker.ietf.org/doc/html/rfc7986) and the underlying iCal format is described at least in [RFC 5545](https://datatracker.ietf.org/doc/html/rfc5545). \
//! This library has been intensivley tested with Nextcloud servers. It should support Owncloud and iCloud as well, since they use the very same CalDAV protocol.
//!
//! This initial implementation only supports TODO events. Thus it can fetch and update a CalDAV-hosted todo-list...just like [sticky notes on a kitchen fridge](https://www.google.com/search?q=kitchen+fridge+todo+list&tbm=isch) would. \
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
//! It also handles synchronisation between the local cache and the server, and robustly recovers from any network error (so that it never corrupts the local or remote source).
//!
//! Note that many methods are defined in common traits (see [`crate::traits`]).
//!
//! ## Examples
//!
//! See example usage in the `examples/` folder, that you can run using `cargo run --example <example-name>`. \
//! You can also have a look at [`Voilà`](https://github.com/daladim/voila-tasks), a GUI app that uses `kitchen-fridge` under the hood.
//!
//! ## Configuration options
//!
//! Have a look at the [`config`] module to see what default options can be overridden.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/daladim/kitchen-fridge/master/resources/kitchen-fridge.svg"
)]

pub mod traits;

pub mod calendar;
pub mod item;
pub use item::Item;
pub mod task;
pub use task::Task;
pub mod event;
pub use event::Event;
pub mod mock_behaviour;
pub mod provider;

pub mod client;
pub use client::Client;
pub mod cache;
pub use cache::Cache;
pub mod ical;

pub mod config;
pub mod resource;
pub mod utils;

/// Unless you want another kind of Provider to write integration tests, you'll probably want this kind of Provider. \
/// See alse the [`Provider` documentation](crate::provider::Provider)
pub type CalDavProvider = provider::Provider<
    cache::Cache,
    calendar::cached_calendar::CachedCalendar,
    Client,
    calendar::remote_calendar::RemoteCalendar,
>;
