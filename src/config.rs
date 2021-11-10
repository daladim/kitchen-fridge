//! Support for compile-time configuration options

use once_cell::sync::Lazy;

/// Part of the ProdID string that describes the organization (example of a ProdID string: `-//ABC Corporation//My Product//EN`)
/// You can override it at compile-time with the `KITCHEN_FRIDGE_ICAL_ORG_NAME` environment variable, or keep the default value
pub static ORG_NAME: Lazy<String> = Lazy::new(|| option_env!("KITCHEN_FRIDGE_ICAL_ORG_NAME").unwrap_or("My organization").to_string() );

/// Part of the ProdID string that describes the product name (example of a ProdID string: `-//ABC Corporation//My Product//EN`)
/// You can override it at compile-time with the `KITCHEN_FRIDGE_ICAL_PRODUCT_NAME` environment variable, or keep the default value
pub static PRODUCT_NAME: Lazy<String> = Lazy::new(|| option_env!("KITCHEN_FRIDGE_ICAL_PRODUCT_NAME").unwrap_or("KitchenFridge").to_string() );
