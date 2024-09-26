use http::{header::CONTENT_TYPE, Method};
use minidom::Element;

use crate::{
    error::{HttpStatusConstraint, KFError, KFResult},
    resource::Resource,
    utils::Namespaces,
};

use super::{
    xml::{find_elem, find_elems},
    NamespacedName,
};

pub(crate) async fn sub_request(
    resource: &Resource,
    method: &str,
    body: String,
    depth: u32,
) -> KFResult<String> {
    let method: Method = method.parse().expect("invalid method name");

    let url = resource.url();

    let res = reqwest::Client::new()
        .request(method.clone(), url.clone())
        .header("Depth", depth)
        .header(CONTENT_TYPE, "application/xml")
        .basic_auth(resource.username(), Some(resource.password()))
        .body(body)
        .send()
        .await
        .map_err(|source| KFError::HttpRequestError {
            url: url.clone(),
            method: method.clone(),
            source,
        })?;

    if !res.status().is_success() {
        return Err(KFError::UnexpectedHTTPStatusCode {
            expected: HttpStatusConstraint::Success,
            got: res.status(),
        });
    }

    let text = res
        .text()
        .await
        .map_err(|source| KFError::HttpRequestError {
            url: url.clone(),
            method,
            source,
        })?;
    Ok(text)
}

pub(crate) async fn sub_request_and_extract_elem(
    resource: &Resource,
    body: String,
    depth: u32,
    items: &[&str],
) -> KFResult<String> {
    let text = sub_request(resource, "PROPFIND", body, depth).await?;

    let mut current_element: &Element = &text
        .parse()
        .map_err(|source| KFError::DOMParseError { text, source })?;
    for item in items {
        current_element = match find_elem(current_element, item) {
            Some(elem) => elem,
            None => {
                return Err(KFError::MissingDOMElement {
                    text: current_element.text(),
                    el: item.to_string(),
                })
            }
        }
    }
    Ok(current_element.text())
}

pub(crate) async fn sub_request_and_extract_elems(
    resource: &Resource,
    method: &str,
    body: String,
    depth: u32,
    item: &str,
) -> KFResult<Vec<Element>> {
    let text = sub_request(resource, method, body, depth).await?;

    let element: &Element = &text
        .parse()
        .map_err(|source| KFError::DOMParseError { text, source })?;
    Ok(find_elems(element, item)
        .iter()
        .map(|elem| (*elem).clone())
        .collect())
}

/// Body of a PROPFIND call that queries the given properties
///
/// This will look something like:
///
/// <d:propfind xmlns:d="DAV:">
///     <d:prop>
///         <d:allprop/>
///     </d:prop>
/// </d:propfind>
pub(crate) fn propfind_body(props: &[NamespacedName]) -> String {
    let mut namespaces = Namespaces::new();
    for p in props {
        namespaces.add(&p.xmlns);
    }

    let prop_names = {
        let mut s = String::new();
        for p in props {
            s.push('<');
            s.push_str(p.with_symbolized_prefix(&namespaces).as_str());
            s.push('/');
            s.push('>');
            s.push('\n');
        }
        s
    };

    let d = namespaces.dav_sym();

    format!(
        r#"
<{}:propfind{}>
    <{}:prop>
{}
    </{}:prop>
</{}:propfind>
"#,
        d,
        namespaces.decl(),
        d,
        prop_names,
        d,
        d,
    )
}
