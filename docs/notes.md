# Notes

RemoteCalendar works directly with a server over HTTP/WebDAV/CalDAV

CachedCalendar mocks this with feature local_calendar_mocks_remote_calendars

tests/syncs.rs separately calculates what it thinks the (mocked) server and client (CachedCalendar) ought to be doing.


