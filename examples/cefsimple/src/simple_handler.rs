use base64::prelude::*;
use cef::*;

pub struct SimpleHandler {
    pub is_alloy_style: bool,
    pub is_closing: bool,
    pub browser_list: Vec<Browser>,
}

#[inline]
fn tid_ui() -> ThreadId {
    ThreadId::from(cef_dll_sys::cef_thread_id_t::TID_UI)
}

#[inline]
fn required_ui_thread() {
    debug_assert_eq!(currently_on(tid_ui()), 1);
}

fn get_data_uri(data: &str, mime_type: &str) -> String {
    format!(
        "data:{mime_type};base64,{}",
        percent_encoding::utf8_percent_encode(
            &BASE64_STANDARD.encode(data),
            percent_encoding::NON_ALPHANUMERIC
        )
    )
}

impl SimpleHandler {
    pub fn new(is_alloy_style: bool) -> Self {
        Self {
            is_alloy_style,
            is_closing: false,
            browser_list: Vec::new(),
        }
    }

    pub fn on_title_change(&self, browser: Option<&mut Browser>, title: Option<&CefString>) {
        required_ui_thread();

        if let Some(browser_view) = browser_view_get_for_browser(browser) {
            if let Some(window) = browser_view.window() {
                window.set_title(title);
            }
        } else if self.is_alloy_style {
            todo!("Implement `PlatformTitleChange`")
        }
    }

    pub fn on_after_created(&mut self, browser: Option<&mut Browser>) {
        required_ui_thread();
        let browser = browser.unwrap();

        let style = if self.is_alloy_style {
            RuntimeStyle::from(cef_dll_sys::cef_runtime_style_t::CEF_RUNTIME_STYLE_ALLOY)
        } else {
            RuntimeStyle::from(cef_dll_sys::cef_runtime_style_t::CEF_RUNTIME_STYLE_CHROME)
        };
        assert_eq!(style, browser.host().unwrap().runtime_style());

        self.browser_list.push(browser.clone());
    }

    pub fn do_close(&mut self, _browser: Option<&mut Browser>) -> bool {
        required_ui_thread();

        if self.browser_list.len() == 1 {
            self.is_closing = true;
        }

        false
    }

    pub fn on_before_close(&mut self, browser: Option<&mut Browser>) {
        required_ui_thread();
        let mut browser = browser.unwrap().clone();

        for (i, bit) in self.browser_list.iter().enumerate() {
            if bit.is_same(Some(&mut browser)) == 1 {
                self.browser_list.remove(i);
                break;
            }
        }

        if self.browser_list.is_empty() {
            // All browser windows have closed.
            // Quit the application message loop.
            quit_message_loop();
        }
    }

    pub fn on_load_error(
        &self,
        _browser: Option<&mut Browser>,
        frame: Option<&mut Frame>,
        error_code: Errorcode,
        error_text: Option<&CefString>,
        failed_url: Option<&CefString>,
    ) {
        required_ui_thread();

        // Allo Chrome to show the error page.
        if !self.is_alloy_style {
            return;
        }

        // Don't display an error for downloaded files.
        if error_code == Errorcode::from(cef_dll_sys::cef_errorcode_t::ERR_ABORTED) {
            return;
        }

        // Display a load error message using a data: URI.
        let html = format!(
            r#"<html>
                <body style="background-color: white;">
                    <h2>Failed to load URL {} with error {} ({}).</h2>
                </body>
            </html>"#,
            failed_url.unwrap(),
            error_text.unwrap(),
            *error_code.as_ref() as i32
        );

        frame
            .unwrap()
            .load_url(Some(&get_data_uri(&html, "text/html").as_str().into()));
    }

    pub fn show_main_window(&mut self) {
        if currently_on(tid_ui()) == 0 {
            todo!("posting task is not yet.");
        }

        if self.browser_list.is_empty() {
            return;
        }

        let browser_view = {
            let main_browser = self.browser_list.first_mut().unwrap();
            browser_view_get_for_browser(Some(main_browser))
        };

        if let Some(browser_view) = browser_view {
            if let Some(window) = browser_view.window() {
                // Show the window using the Views framework.
                window.show();
            }
        } else if self.is_alloy_style {
            self.platform_show_window(self.browser_list.first().unwrap());
        };
    }

    #[cfg(not(target_os = "macos"))]
    fn platform_show_window(&self, _browser: &Browser) {
        unimplemented!()
    }

    #[cfg(target_os = "macos")]
    fn platform_show_window(&self, browser: &Browser) {
        use objc2::rc::Retained;
        use objc2_app_kit::{NSView, NSWindow};

        fn get_ns_window_for_browser(browser: &Browser) -> Retained<NSWindow> {
            let ptr = browser.host().unwrap().window_handle() as *mut NSView;
            unsafe { Retained::retain(ptr).unwrap().window().unwrap() }
        }

        let window = get_ns_window_for_browser(browser);
        window.makeKeyAndOrderFront(Some(window.as_ref()));
    }

    pub fn close_all_browsers(&self, force_close: bool) {
        if currently_on(tid_ui()) == 0 {
            todo!("posting task is not yet.")
        }

        if self.browser_list.is_empty() {
            return;
        }

        for browser in self.browser_list.iter() {
            browser.host().unwrap().close_browser(force_close as _);
        }
    }
}
