//! A saved sign-in on its way from the person's vault into the page they are
//! looking at.
//!
//! It only ever goes to the site it was saved for: an encrypted page whose
//! host is the saved one. The shell checks that against the address it holds
//! for the tab, and the script checks again in the document it runs in, so a
//! page that navigated in between gets nothing. The values are wiped from
//! memory when dropped and never printed.

use std::fmt;

use url::Url;
use zeroize::Zeroizing;

const MAX_USERNAME_BYTES: usize = 1024;
const MAX_PASSWORD_BYTES: usize = 4096;

#[derive(Clone)]
pub struct LoginFill {
    host: String,
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

/// The host a sign-in belongs to: the address's own, lower case, without a
/// leading `www.`; none unless the address is encrypted.
fn site_of(address: &Url) -> Option<String> {
    if address.scheme() != "https" {
        return None;
    }
    let host = address.host_str()?.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    (!host.is_empty()).then(|| host.to_owned())
}

impl LoginFill {
    /// None unless the saved address is an encrypted site's and there is a
    /// password to fill, with both values bounded and free of NUL.
    pub fn new(address: &str, username: String, password: String) -> Option<Self> {
        let (username, password) = (Zeroizing::new(username), Zeroizing::new(password));
        let host = site_of(&Url::parse(address.trim()).ok()?)?;
        (!password.is_empty()
            && username.len() <= MAX_USERNAME_BYTES
            && password.len() <= MAX_PASSWORD_BYTES
            && !username.contains('\0')
            && !password.contains('\0'))
        .then_some(Self {
            host,
            username,
            password,
        })
    }

    /// Whether `page` is the site this sign-in was saved for.
    pub fn belongs_on(&self, page: &Url) -> bool {
        site_of(page).is_some_and(|host| host == self.host)
    }

    /// The expression that fills the sign-in into the document it runs in,
    /// and yields whether it found a field. It does nothing anywhere but on
    /// the saved site. The name goes in the text field before the password,
    /// or in a lone name field on a sign-in that asks in two steps.
    pub fn script(&self) -> Zeroizing<String> {
        let quoted = |text: &str| Zeroizing::new(serde_json::Value::from(text).to_string());
        let (host, name, secret) = (
            quoted(&self.host),
            quoted(&self.username),
            quoted(&self.password),
        );
        let values = [
            ("__HOST__", host.as_str()),
            ("__NAME__", name.as_str()),
            ("__SECRET__", secret.as_str()),
        ];
        // One pass over the template, with room for all of it up front: a
        // value is never read for markers, and no copy is left behind by a
        // buffer that had to grow.
        let room = SCRIPT.len() + 2 * (host.len() + name.len() + secret.len());
        let mut out = Zeroizing::new(String::with_capacity(room));
        let mut rest = SCRIPT;
        'template: while let Some(first) = rest.chars().next() {
            for (marker, value) in values {
                if let Some(after) = rest.strip_prefix(marker) {
                    out.push_str(value);
                    rest = after;
                    continue 'template;
                }
            }
            out.push(first);
            rest = &rest[first.len_utf8()..];
        }
        out
    }
}

impl fmt::Debug for LoginFill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LoginFill(redacted)")
    }
}

const SCRIPT: &str = r#"(() => {
  "use strict";
  if (location.protocol !== "https:") return false;
  if (location.hostname.toLowerCase().replace(/^www\./, "") !== __HOST__) return false;
  const shown = (field) => {
    const box = field.getBoundingClientRect();
    return box.width > 0 && box.height > 0 && getComputedStyle(field).visibility !== "hidden";
  };
  const typed = (field) => ["text", "email", "tel", ""].includes(field.type) && !field.readOnly;
  const fields = Array.from(document.querySelectorAll("input")).filter(shown);
  const secret = fields.find((field) => field.type === "password" && !field.readOnly) || null;
  const named = (field) =>
    /user|e-?mail|login|ident|account|compte/i.test(`${field.autocomplete} ${field.name} ${field.id}`);
  const name = secret
    ? fields.slice(0, fields.indexOf(secret)).filter(typed).pop() || null
    : fields.find((field) => typed(field) && named(field)) || null;
  const write = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
  const put = (field, text) => {
    field.focus();
    write.call(field, text);
    field.dispatchEvent(new Event("input", { bubbles: true }));
    field.dispatchEvent(new Event("change", { bubbles: true }));
  };
  if (name && __NAME__) put(name, __NAME__);
  if (secret) put(secret, __SECRET__);
  return Boolean(name || secret);
})()"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn fill(address: &str) -> Option<LoginFill> {
        LoginFill::new(address, "me@site.test".into(), "hunter2".into())
    }

    #[test]
    fn a_sign_in_goes_only_to_the_encrypted_site_it_was_saved_for() {
        let saved = fill("https://www.Site.test/login?next=1").expect("fill");
        for page in [
            "https://site.test/",
            "https://www.site.test/account",
            "https://SITE.test:443/",
        ] {
            assert!(saved.belongs_on(&Url::parse(page).unwrap()), "{page}");
        }
        for page in [
            "http://site.test/",
            "https://evil.test/site.test",
            "https://site.test.evil.test/",
            "https://login.site.test/",
            "https://notsite.test/",
            "file:///C:/site.test",
            "about:blank",
        ] {
            assert!(!saved.belongs_on(&Url::parse(page).unwrap()), "{page}");
        }
    }

    #[test]
    fn nothing_is_made_without_an_encrypted_address_and_a_password() {
        assert!(fill("http://site.test/").is_none());
        assert!(fill("site.test").is_none());
        assert!(fill("").is_none());
        assert!(fill("javascript:alert(1)").is_none());
        assert!(LoginFill::new("https://site.test", "me".into(), String::new()).is_none());
        assert!(LoginFill::new("https://site.test", "me".into(), "a\0b".into()).is_none());
        assert!(LoginFill::new("https://site.test", "x".repeat(1025), "p".into()).is_none());
        assert!(LoginFill::new("https://site.test", "me".into(), "p".repeat(4097)).is_none());
        // A password alone is a sign-in: some sites ask for nothing else.
        assert!(LoginFill::new("https://site.test", String::new(), "p".into()).is_some());
    }

    #[test]
    fn values_reach_the_script_as_text_and_never_as_code() {
        let hostile = LoginFill::new(
            "https://site.test",
            "\"); alert(1); (\"".into(),
            "</script>\\\u{2028}'`${x}".into(),
        )
        .expect("fill");
        let script = hostile.script();
        // Each value is one JSON string: its quotes and backslashes escaped.
        assert!(script.contains(r#""\"); alert(1); (\"""#));
        assert!(script.contains(r#""</script>\\"#));
        assert!(!script.contains("__NAME__") && !script.contains("__SECRET__"));
        assert!(script.contains(r#"!== "site.test""#));
    }

    #[test]
    fn a_value_that_spells_a_marker_stays_text() {
        let odd = LoginFill::new("https://site.test", "__SECRET__".into(), "\"+x+\"".into())
            .expect("fill");
        let script = odd.script();
        assert!(script.contains(r#"put(name, "__SECRET__")"#));
        assert!(script.contains(r#"put(secret, "\"+x+\"")"#));
    }

    #[test]
    fn it_is_never_printed() {
        let text = format!("{:?}", fill("https://site.test").expect("fill"));
        assert_eq!(text, "LoginFill(redacted)");
        assert!(!text.contains("hunter2") && !text.contains("me@site.test"));
    }
}
