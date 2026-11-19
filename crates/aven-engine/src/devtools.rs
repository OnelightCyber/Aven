//! A request to a view's own DevTools channel that expects no answer. The
//! browser carries it out: nothing is evaluated in the page.

/// A request whose answer is wanted. `done` gets the result as JSON text, or
/// none when the browser refused it. When the request cannot even be made,
/// `done` is dropped unrun: a caller waiting on it sees its channel close.
#[cfg(target_os = "windows")]
pub(crate) fn call(
    core: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2,
    method: &str,
    parameters: &str,
    done: impl FnOnce(Option<String>) + 'static,
) {
    use webview2_com::CallDevToolsProtocolMethodCompletedHandler;
    use windows::core::HSTRING;

    let handler =
        CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |status, response| {
            done(status.is_ok().then_some(response));
            Ok(())
        }));
    let _ = unsafe {
        core.CallDevToolsProtocolMethod(
            &HSTRING::from(method),
            &HSTRING::from(parameters),
            &handler,
        )
    };
}

#[cfg(target_os = "windows")]
pub(crate) fn send(view: &wry::WebView, method: &str, parameters: &str) {
    use webview2_com::CallDevToolsProtocolMethodCompletedHandler;
    use windows::core::HSTRING;
    use wry::WebViewExtWindows;

    let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(|_, _| Ok(())));
    // A view that is closing keeps what it had: there is nothing to show.
    let _ = unsafe {
        view.webview().CallDevToolsProtocolMethod(
            &HSTRING::from(method),
            &HSTRING::from(parameters),
            &handler,
        )
    };
}
