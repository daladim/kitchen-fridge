///! Some utility functions

use minidom::Element;
use serde::Deserialize;

/// Walks an XML tree and returns every element that has the given name
pub fn find_elems<S: AsRef<str>>(root: &Element, searched_name: S) -> Vec<&Element> {
    let searched_name = searched_name.as_ref();
    let mut elems: Vec<&Element> = Vec::new();

    for el in root.children() {
        if el.name() == searched_name {
            elems.push(el);
        } else {
            let ret = find_elems(el, searched_name);
            elems.extend(ret);
        }
    }
    elems
}

/// Walks an XML tree until it finds an elements with the given name
pub fn find_elem<S: AsRef<str>>(root: &Element, searched_name: S) -> Option<&Element> {
    let searched_name = searched_name.as_ref();
    if root.name() == searched_name {
        return Some(root);
    }

    for el in root.children() {
        if el.name() == searched_name {
            return Some(el);
        } else {
            let ret = find_elem(el, searched_name);
            if ret.is_some() {
                return ret;
            }
        }
    }
    None
}

pub fn print_xml(element: &Element) {
    use std::io::Write;
    let mut writer = std::io::stdout();

    let mut xml_writer = minidom::quick_xml::Writer::new_with_indent(
        std::io::stdout(),
        0x20, 4
    );
    element.to_writer(&mut xml_writer);

    writer.write(&[0x0a]);
}


/// Used to (de)serialize url::Url
pub mod url_serde{
    use super::*;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<url::Url, D::Error>
    where
        D: serde::de::Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        match url::Url::parse(&s) {
            Ok(u) => Ok(u),
            Err(_) => { return Err(serde::de::Error::invalid_value(serde::de::Unexpected::Str(&s), &"Expected an url")); }
        }
    }

    pub fn serialize<S>(u: &url::Url, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str( u.as_str() )
    }
}
