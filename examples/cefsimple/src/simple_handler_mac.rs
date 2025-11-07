use cef::*;
use objc2::rc::Retained;
use objc2_app_kit::{NSView, NSWindow};
use objc2_foundation::NSString;

fn get_ns_window_for_browser(browser: &Browser) -> Retained<NSWindow> {
    let ptr = browser.host().unwrap().window_handle() as *mut NSView;
    unsafe { Retained::retain(ptr).unwrap().window().unwrap() }
}

pub fn platform_title_change(browser: &Browser, title: &CefString) {
    let window = get_ns_window_for_browser(browser);
    window.setTitle(&NSString::from_str(&title.to_string()));
}

pub fn platform_show_window(browser: &Browser) {
    let window = get_ns_window_for_browser(browser);
    window.makeKeyAndOrderFront(Some(window.as_ref()));
}
