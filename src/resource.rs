use url::Url;

/// Just a wrapper around a URL and credentials
#[derive(Clone, Debug)]
pub struct Resource {
    url: Url,
    username: String,
    password: String,
}

impl Resource {
    pub fn new(url: Url, username: String, password: String) -> Self {
        Self {
            url,
            username,
            password,
        }
    }

    pub fn url(&self) -> &Url {
        &self.url
    }
    pub fn username(&self) -> &String {
        &self.username
    }
    pub fn password(&self) -> &String {
        &self.password
    }

    /// Build a new Resource by keeping the same credentials, scheme and server from `base` but changing the path part
    pub fn combine(&self, new_path: &str) -> Resource {
        let mut built = (*self).clone();
        built.url.set_path(&new_path);
        built
    }
}
