//! A picture of what a page view shows, for the person to hand to a model.
//! WebView2 writes the JPEG straight into a bounded stream owned here. A view
//! that is hidden, oversized or over the limit yields nothing.

use super::semantic_screenshot::{BoundedCaptureStream, SharedCaptureStream};
use crate::platform::preview_image;
use std::{cell::RefCell, rc::Rc};
use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_JPEG;
use windows::Win32::System::Com::IStream;
use wry::WebViewExtWindows;

/// The most a picture may weigh: what the assistant admits, and under what
/// model providers accept.
const MAX_BYTES: usize = 4 * 1024 * 1024;

type Completion = Box<dyn FnOnce(Option<Vec<u8>>) + Send>;

/// The tallest a whole-page picture goes, in page pixels, and the widest it
/// is drawn, in picture pixels: enough for a model to read a long page
/// without the picture outgrowing what it is allowed to weigh.
const WHOLE_MAX_HEIGHT: f64 = 6_000.0;
const WHOLE_MAX_WIDTH: f64 = 1_600.0;

/// What `Page.captureScreenshot` is sent for a whole page of the size
/// `Page.getLayoutMetrics` reported, or none when that size makes no sense.
pub(crate) fn whole_request(metrics: &serde_json::Value) -> Option<String> {
    let size = metrics.get("cssContentSize")?;
    let width = size.get("width")?.as_f64()?;
    let height = size.get("height")?.as_f64()?;
    if !(width.is_finite() && height.is_finite() && width >= 1.0 && height >= 1.0) {
        return None;
    }
    let width = width.min(8_192.0);
    Some(
        serde_json::json!({
            "format": "jpeg",
            "quality": 72,
            "captureBeyondViewport": true,
            "clip": {
                "x": 0,
                "y": 0,
                "width": width,
                "height": height.min(WHOLE_MAX_HEIGHT),
                "scale": (WHOLE_MAX_WIDTH / width).min(1.0),
            },
        })
        .to_string(),
    )
}

/// The JPEG in a `Page.captureScreenshot` answer, when it is one and
/// within the limit.
pub(crate) fn whole_picture(answer: &str) -> Option<Vec<u8>> {
    use base64::Engine as _;
    let answer: serde_json::Value = serde_json::from_str(answer).ok()?;
    let data = answer.get("data")?.as_str()?;
    if data.len() > MAX_BYTES / 3 * 4 + 4 {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .ok()?;
    (bytes.starts_with(&[0xFF, 0xD8]) && bytes.len() <= MAX_BYTES).then_some(bytes)
}

/// Reads what is typed in the page's sign-in form: the last filled password
/// field on screen and the text field before it. Run only because the
/// person asked to save it, and only on an encrypted page. It answers with
/// JSON text naming the page's own host, which the caller checks against
/// the address it holds for the tab.
pub(crate) const TYPED_LOGIN: &str = r#"(() => {
  "use strict";
  if (location.protocol !== "https:") return null;
  const shown = (field) => {
    const box = field.getBoundingClientRect();
    return box.width > 0 && box.height > 0 && getComputedStyle(field).visibility !== "hidden";
  };
  const fields = Array.from(document.querySelectorAll("input")).filter(shown);
  const secret = fields.filter((field) => field.type === "password" && field.value).pop();
  if (!secret) return null;
  const typed = (field) => ["text", "email", "tel", ""].includes(field.type) && field.value;
  const name = fields.slice(0, fields.indexOf(secret)).filter(typed).pop();
  return JSON.stringify({
    host: location.hostname,
    name: name ? String(name.value).slice(0, 1024) : "",
    secret: String(secret.value).slice(0, 4096),
  });
})()"#;

pub(crate) fn capture(view: &wry::WebView, done: Completion) {
    let mut bounds = windows::Win32::Foundation::RECT::default();
    let valid = unsafe { view.controller().Bounds(&mut bounds) }.is_ok();
    let dimensions = (
        bounds
            .right
            .checked_sub(bounds.left)
            .and_then(|v| u32::try_from(v).ok()),
        bounds
            .bottom
            .checked_sub(bounds.top)
            .and_then(|v| u32::try_from(v).ok()),
    );
    let (Some(width), Some(height)) = dimensions else {
        done(None);
        return;
    };
    if !valid || !preview_image::input_dimensions(width, height) {
        done(None);
        return;
    }
    let shared = Rc::new(SharedCaptureStream::new(MAX_BYTES));
    let stream: IStream = BoundedCaptureStream {
        shared: shared.clone(),
    }
    .into();
    let pending = Rc::new(RefCell::new(Some((done, shared, stream.clone()))));
    let callback = pending.clone();
    let handler = webview2_com::CapturePreviewCompletedHandler::create(Box::new(move |result| {
        let Some((done, shared, _stream)) = callback.borrow_mut().take() else {
            return Ok(());
        };
        if result.is_err() || shared.failure.get().is_some() {
            done(None);
            return Ok(());
        }
        let bytes = shared.buffer.borrow_mut().take_bytes();
        done((!bytes.is_empty()).then_some(bytes));
        Ok(())
    }));
    let mut visible = windows_core::BOOL::default();
    if unsafe { view.controller().IsVisible(&mut visible) }.is_err()
        || !visible.as_bool()
        || unsafe {
            view.webview().CapturePreview(
                COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_JPEG,
                &stream,
                &handler,
            )
        }
        .is_err()
    {
        if let Some((done, _, _)) = pending.borrow_mut().take() {
            done(None);
        }
    }
}
