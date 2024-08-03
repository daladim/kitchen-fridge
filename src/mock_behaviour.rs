//! This module provides ways to tweak mocked calendars, so that they can return errors on some tests
#![cfg(feature = "local_calendar_mocks_remote_calendars")]

use std::error::Error;

/// This stores some behaviour tweaks, that describe how a mocked instance will behave during a given test
///
/// So that a functions fails _n_ times after _m_ initial successes, set `(m, n)` for the suited parameter
#[derive(Default, Clone, Debug)]
pub struct MockBehaviour {
    /// If this is true, every action will be allowed
    pub is_suspended: bool,

    // From the CalDavSource trait
    pub get_calendars_behaviour: (u32, u32),
    //pub get_calendar_behaviour: (u32, u32),
    pub create_calendar_behaviour: (u32, u32),

    // From the BaseCalendar trait
    pub add_item_behaviour: (u32, u32),
    pub update_item_behaviour: (u32, u32),

    // From the DavCalendar trait
    pub get_item_version_tags_behaviour: (u32, u32),
    pub get_item_by_url_behaviour: (u32, u32),
    pub delete_item_behaviour: (u32, u32),
}

impl MockBehaviour {
    pub fn new() -> Self {
        Self::default()
    }

    /// All items will fail at once, for `n_fails` times
    pub fn fail_now(n_fails: u32) -> Self {
        Self {
            is_suspended: false,
            get_calendars_behaviour: (0, n_fails),
            //get_calendar_behaviour: (0, n_fails),
            create_calendar_behaviour: (0, n_fails),
            add_item_behaviour: (0, n_fails),
            update_item_behaviour: (0, n_fails),
            get_item_version_tags_behaviour: (0, n_fails),
            get_item_by_url_behaviour: (0, n_fails),
            delete_item_behaviour: (0, n_fails),
        }
    }

    /// Suspend this mock behaviour until you call `resume`
    pub fn suspend(&mut self) {
        self.is_suspended = true;
    }
    /// Make this behaviour active again
    pub fn resume(&mut self) {
        self.is_suspended = false;
    }

    pub fn copy_from(&mut self, other: &Self) {
        self.get_calendars_behaviour = other.get_calendars_behaviour;
        self.create_calendar_behaviour = other.create_calendar_behaviour;
    }

    pub fn can_get_calendars(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_suspended {
            return Ok(());
        }
        decrement(&mut self.get_calendars_behaviour, "get_calendars")
    }
    // pub fn can_get_calendar(&mut self) -> Result<(), Box<dyn Error>> {
    //     if self.is_suspended { return Ok(()) }
    //     decrement(&mut self.get_calendar_behaviour, "get_calendar")
    // }
    pub fn can_create_calendar(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_suspended {
            return Ok(());
        }
        decrement(&mut self.create_calendar_behaviour, "create_calendar")
    }
    pub fn can_add_item(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_suspended {
            return Ok(());
        }
        decrement(&mut self.add_item_behaviour, "add_item")
    }
    pub fn can_update_item(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_suspended {
            return Ok(());
        }
        decrement(&mut self.update_item_behaviour, "update_item")
    }
    pub fn can_get_item_version_tags(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_suspended {
            return Ok(());
        }
        decrement(
            &mut self.get_item_version_tags_behaviour,
            "get_item_version_tags",
        )
    }
    pub fn can_get_item_by_url(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_suspended {
            return Ok(());
        }
        decrement(&mut self.get_item_by_url_behaviour, "get_item_by_url")
    }
    pub fn can_delete_item(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_suspended {
            return Ok(());
        }
        decrement(&mut self.delete_item_behaviour, "delete_item")
    }
}

/// Return Ok(()) in case the value is `(1+, _)` or `(_, 0)`, or return Err and decrement otherwise
fn decrement(value: &mut (u32, u32), descr: &str) -> Result<(), Box<dyn Error>> {
    let remaining_successes = value.0;
    let remaining_failures = value.1;

    if remaining_successes > 0 {
        value.0 = value.0 - 1;
        log::debug!("Mock behaviour: allowing a {} ({:?})", descr, value);
        Ok(())
    } else {
        if remaining_failures > 0 {
            value.1 = value.1 - 1;
            log::debug!("Mock behaviour: failing a {} ({:?})", descr, value);
            Err(format!(
                "Mocked behaviour requires this {} to fail this time. ({:?})",
                descr, value
            )
            .into())
        } else {
            log::debug!("Mock behaviour: allowing a {} ({:?})", descr, value);
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_mock_behaviour() {
        let mut ok = MockBehaviour::new();
        assert!(ok.can_get_calendars().is_ok());
        assert!(ok.can_get_calendars().is_ok());
        assert!(ok.can_get_calendars().is_ok());
        assert!(ok.can_get_calendars().is_ok());
        assert!(ok.can_get_calendars().is_ok());
        assert!(ok.can_get_calendars().is_ok());
        assert!(ok.can_get_calendars().is_ok());

        let mut now = MockBehaviour::fail_now(2);
        assert!(now.can_get_calendars().is_err());
        assert!(now.can_create_calendar().is_err());
        assert!(now.can_create_calendar().is_err());
        assert!(now.can_get_calendars().is_err());
        assert!(now.can_get_calendars().is_ok());
        assert!(now.can_get_calendars().is_ok());
        assert!(now.can_create_calendar().is_ok());

        let mut custom = MockBehaviour {
            get_calendars_behaviour: (0, 1),
            create_calendar_behaviour: (1, 3),
            ..MockBehaviour::default()
        };
        assert!(custom.can_get_calendars().is_err());
        assert!(custom.can_get_calendars().is_ok());
        assert!(custom.can_get_calendars().is_ok());
        assert!(custom.can_get_calendars().is_ok());
        assert!(custom.can_get_calendars().is_ok());
        assert!(custom.can_get_calendars().is_ok());
        assert!(custom.can_get_calendars().is_ok());
        assert!(custom.can_create_calendar().is_ok());
        assert!(custom.can_create_calendar().is_err());
        assert!(custom.can_create_calendar().is_err());
        assert!(custom.can_create_calendar().is_err());
        assert!(custom.can_create_calendar().is_ok());
        assert!(custom.can_create_calendar().is_ok());
    }
}
