//! A short list of sites the person set apart from a setting that otherwise
//! applies to every page, kept as a preference: one host a line.
//!
//! A host covers its own pages and those of the names under it, and a
//! leading `www.` never matters. Nothing but hosts is kept: no paths, no
//! schemes, no patterns.

use url::Url;

/// Most sites one list holds.
pub const MAX_SITES: usize = 200;
/// The most a list's text may weigh.
pub const MAX_BYTES: usize = 16 * 1024;

/// The sites of one list, as they are matched.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SiteList(Vec<String>);

/// A host as the list keeps it: lower case, without a leading `www.`.
fn host(line: &str) -> Option<String> {
    let line = line.trim().to_ascii_lowercase();
    let name = line.strip_prefix("www.").unwrap_or(&line);
    let fine = !name.is_empty()
        && name.len() <= 253
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.')
        && !name.starts_with(['.', '-'])
        && !name.ends_with(['.', '-'])
        && !name.contains("..");
    fine.then(|| name.to_owned())
}

impl SiteList {
    /// The list a stored preference holds. A line that is not a host is
    /// dropped, and a host named twice counts once.
    pub fn parse(value: &str) -> Self {
        let mut hosts: Vec<String> = Vec::new();
        for line in value.lines() {
            if hosts.len() >= MAX_SITES {
                break;
            }
            if let Some(host) = host(line).filter(|host| !hosts.contains(host)) {
                hosts.push(host);
            }
        }
        Self(hosts)
    }

    /// Whether `value` may be stored: bounded, and every line a host.
    pub fn allowed(value: &str) -> bool {
        value.len() <= MAX_BYTES
            && value.lines().count() <= MAX_SITES
            && value.lines().all(|line| host(line).is_some())
    }

    /// Whether the page at `page` is on one of the sites.
    pub fn covers(&self, page: &Url) -> bool {
        let Some(name) = page.host_str() else {
            return false;
        };
        let name = name.to_ascii_lowercase();
        let name = name.strip_prefix("www.").unwrap_or(&name);
        self.0.iter().any(|site| {
            name == site
                || name
                    .strip_suffix(site.as_str())
                    .is_some_and(|rest| rest.ends_with('.'))
        })
    }

    /// The hosts, as the engine is told them.
    pub fn hosts(&self) -> &[String] {
        &self.0
    }

    /// No site is set apart.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(address: &str) -> Url {
        Url::parse(address).expect("an address")
    }

    #[test]
    fn a_host_covers_itself_and_the_names_under_it() {
        let list = SiteList::parse("Example.com\nwww.news.example\n");
        assert_eq!(list.hosts(), ["example.com", "news.example"]);
        assert!(list.covers(&page("https://example.com/a")));
        assert!(list.covers(&page("https://www.example.com/")));
        assert!(list.covers(&page("http://shop.example.com:8080/x")));
        assert!(list.covers(&page("https://news.example/")));
        assert!(!list.covers(&page("https://notexample.com/")));
        assert!(!list.covers(&page("https://example.com.evil.test/")));
        assert!(!list.covers(&page("about:blank")));
    }

    #[test]
    fn only_hosts_are_kept_and_each_once() {
        let list =
            SiteList::parse("example.com\nhttps://example.org/a\n\nexample.com\n a b \n-x.test\n");
        assert_eq!(list.hosts(), ["example.com"]);
        assert!(SiteList::parse("").is_empty());
    }

    #[test]
    fn a_stored_value_is_bounded_and_made_of_hosts_only() {
        assert!(SiteList::allowed(""));
        assert!(SiteList::allowed("example.com\nlocalhost"));
        assert!(!SiteList::allowed("example.com\nhttps://example.org"));
        assert!(!SiteList::allowed("example.com\n\nexample.org"));
        let many = vec!["a.test"; MAX_SITES + 1].join("\n");
        assert!(!SiteList::allowed(&many));
        assert!(!SiteList::allowed(&"a".repeat(MAX_BYTES + 1)));
    }

    #[test]
    fn a_list_never_grows_past_its_limit() {
        let many: String = (0..MAX_SITES + 20)
            .map(|n| format!("s{n}.test\n"))
            .collect();
        assert_eq!(SiteList::parse(&many).hosts().len(), MAX_SITES);
    }
}
