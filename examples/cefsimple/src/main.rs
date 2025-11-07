mod simple_handler;
#[cfg(target_os = "macos")]
mod simple_handler_mac;

use cef::{args::Args, rc::*, *};
use std::sync::{Arc, Mutex};

use crate::simple_handler::SimpleHandler;

wrap_app! {
    struct DemoApp {
        simple_handler: Arc<Mutex<SimpleHandler>>,
        window: Arc<Mutex<Option<Window>>>,
    }

    impl App {
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(DemoBrowserProcessHandler::new(
                self.simple_handler.clone(), self.window.clone(),
            ))
        }
    }
}

wrap_browser_process_handler! {
    struct DemoBrowserProcessHandler {
        simple_handler: Arc<Mutex<SimpleHandler>>,
        window: Arc<Mutex<Option<Window>>>,
    }

    impl BrowserProcessHandler {
        // The real lifespan of cef starts from `on_context_initialized`, so all the cef objects should be manipulated after that.
        fn on_context_initialized(&self) {
            println!("cef context intiialized");

            let display_handler = DemoDisplayHandler::new(self.simple_handler.clone());
            let life_span_handler = DemoLifeSpanHandler::new(self.simple_handler.clone());
            let load_handler =  DemoLoadHandler::new(self.simple_handler.clone());

            let mut client = DemoClient::new(display_handler, life_span_handler, load_handler);
            let url = CefString::from("https://www.google.com");

            let browser_view = browser_view_create(
                Some(&mut client),
                Some(&url),
                Some(&Default::default()),
                Option::<&mut DictionaryValue>::None,
                Option::<&mut RequestContext>::None,
                Option::<&mut BrowserViewDelegate>::None,
            )
            .expect("Failed to create browser view");

            let mut delegate = DemoWindowDelegate::new(browser_view);
            if let Ok(mut window) = self.window.lock() {
                *window = Some(
                    window_create_top_level(Some(&mut delegate)).expect("Failed to create window"),
                );
            }
        }
    }
}

wrap_life_span_handler! {
    struct DemoLifeSpanHandler {
        simple_handler: Arc<Mutex<SimpleHandler>>
    }

    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            self.simple_handler.lock().unwrap().on_after_created(browser);
        }

        fn do_close(&self, browser: Option<&mut Browser>) -> ::std::os::raw::c_int {
            self.simple_handler.lock().unwrap().do_close(browser) as _
        }

        fn on_before_close(&self, browser: Option<&mut Browser>) {
            self.simple_handler.lock().unwrap().on_before_close(browser);
        }
    }
}

wrap_load_handler! {
    struct DemoLoadHandler {
        simple_handler: Arc<Mutex<SimpleHandler>>
    }

    impl LoadHandler {
        fn on_load_error(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            error_code: Errorcode,
            error_text: Option<&CefString>,
            failed_url: Option<&CefString>,
        ) {
            self.simple_handler.lock().unwrap().on_load_error(
                browser,
                frame,
                error_code,
                error_text,
                failed_url
            )
        }
    }
}

wrap_display_handler! {
    struct DemoDisplayHandler {
        simple_handler: Arc<Mutex<SimpleHandler>>
    }

    impl DisplayHandler {
        fn on_title_change(
            &self,
            browser: Option<&mut Browser>,
            title: Option<&CefString>
        ) {
            self.simple_handler.lock().unwrap().on_title_change(browser, title);
        }
    }
}

wrap_client! {
    struct DemoClient {
        display_handler: DisplayHandler,
        life_span_handler: LifeSpanHandler,
        load_handler: LoadHandler
    }

    impl Client {
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(self.display_handler.clone())
        }

        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(self.life_span_handler.clone())
        }

        fn load_handler(&self) -> Option<LoadHandler> {
            Some(self.load_handler.clone())
        }
    }
}

wrap_window_delegate! {
    struct DemoWindowDelegate {
        browser_view: BrowserView,
    }

    impl ViewDelegate {
        fn on_child_view_changed(
            &self,
            _view: Option<&mut View>,
            _added: ::std::os::raw::c_int,
            _child: Option<&mut View>,
        ) {
            // view.as_panel().map(|x| x.as_window().map(|w| w.close()));
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            if let Some(window) = window {
                let view = self.browser_view.clone();
                window.add_child_view(Some(&mut (&view).into()));
                window.show();
            }
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            quit_message_loop();
        }

        fn with_standard_window_buttons(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            1
        }

        fn can_resize(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            1
        }

        fn can_maximize(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            1
        }

        fn can_minimize(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            1
        }

        fn can_close(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            1
        }
    }
}

// FIXME: Rewrite this demo based on cef/tests/cefsimple
fn main() {
    #[cfg(target_os = "macos")]
    let _loader = {
        let loader = library_loader::LibraryLoader::new(&std::env::current_exe().unwrap(), false);
        assert!(loader.load());
        loader
    };

    #[cfg(target_os = "macos")]
    {
        use objc2::{
            ClassType, MainThreadMarker, msg_send,
            rc::Retained,
            runtime::{AnyObject, NSObjectProtocol},
        };
        use objc2_app_kit::NSApp;

        use application::SimpleApplication;

        let mtm = MainThreadMarker::new().unwrap();

        unsafe {
            // Initialize the SimpleApplication instance.
            // SAFETY: mtm ensures that here is the main thread.
            let _: Retained<AnyObject> = msg_send![SimpleApplication::class(), sharedApplication];
        }

        // If there was an invocation to NSApp prior to here,
        // then the NSApp will not be a SimpleApplication.
        // The following assertion ensures that this doesn't happen.
        assert!(NSApp(mtm).isKindOfClass(SimpleApplication::class()));
    }

    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    let args = Args::new();
    let cmd = args.as_cmd_line().unwrap();

    let switch = CefString::from("type");
    let is_browser_process = cmd.has_switch(Some(&switch)) != 1;

    let window = Arc::new(Mutex::new(None));
    let simple_handler = Arc::new(Mutex::new(SimpleHandler::new(false)));
    let mut app = DemoApp::new(simple_handler.clone(), window.clone());

    let ret = execute_process(
        Some(args.as_main_args()),
        Some(&mut app),
        std::ptr::null_mut(),
    );

    if is_browser_process {
        println!("launch browser process");
        assert!(ret == -1, "cannot execute browser process");
    } else {
        let process_type = CefString::from(&cmd.switch_value(Some(&switch)));
        println!("launch process {process_type}");
        assert!(ret >= 0, "cannot execute non-browser process");
        // non-browser process does not initialize cef
        return;
    }
    let settings = Settings {
        no_sandbox: !cfg!(feature = "sandbox") as _,
        ..Default::default()
    };
    assert_eq!(
        initialize(
            Some(args.as_main_args()),
            Some(&settings),
            Some(&mut app),
            std::ptr::null_mut(),
        ),
        1
    );

    #[cfg(target_os = "macos")]
    let _delegate = {
        use objc2::{runtime::ProtocolObject, sel};
        use objc2_app_kit::NSApp;
        use objc2_foundation::NSObjectNSThreadPerformAdditions;

        use crate::application::SimpleAppDelegate;

        let mtm = objc2::MainThreadMarker::new().unwrap();

        let delegate = SimpleAppDelegate::new(mtm, simple_handler);
        NSApp(mtm).setDelegate(Some(&ProtocolObject::from_retained(delegate.clone())));

        unsafe {
            delegate.performSelectorOnMainThread_withObject_waitUntilDone(
                sel!(createApplication:),
                None,
                false,
            );
        }

        // The delegate property of `NSApplication` is weak property, so retain this.
        delegate
    };

    run_message_loop();

    let window = window.lock().expect("Failed to lock window");
    let window = window.as_ref().expect("Window is None");
    assert!(window.has_one_ref());

    shutdown();
}

#[cfg(target_os = "macos")]
mod application {
    use std::{
        cell::Cell,
        sync::{Arc, Mutex},
    };

    use cef::application_mac::{CefAppProtocol, CrAppControlProtocol, CrAppProtocol};
    use objc2::{rc::Retained, runtime::*, *};
    use objc2_app_kit::*;
    use objc2_foundation::*;

    use crate::SimpleHandler;

    /// Instance variables of `SimpleApplication`.
    pub struct SimpleApplicationIvars {
        handling_send_event: Cell<Bool>,
    }

    define_class!(
        /// A `NSApplication` subclass that implements the required CEF protocols.
        ///
        /// This class provides the necessary `CefAppProtocol` conformance to
        /// ensure that events are handled correctly by the Chromium framework on macOS.
        #[unsafe(super(NSApplication))]
        #[ivars = SimpleApplicationIvars]
        pub struct SimpleApplication;

        unsafe impl CrAppControlProtocol for SimpleApplication {
            #[unsafe(method(setHandlingSendEvent:))]
            unsafe fn set_handling_send_event(&self, handling_send_event: Bool) {
                self.ivars().handling_send_event.set(handling_send_event);
            }
        }

        unsafe impl CrAppProtocol for SimpleApplication {
            #[unsafe(method(isHandlingSendEvent))]
            unsafe fn is_handling_send_event(&self) -> Bool {
                self.ivars().handling_send_event.get()
            }
        }

        unsafe impl CefAppProtocol for SimpleApplication {}

        impl SimpleApplication {
            #[unsafe(method(sendEvent:))]
            fn send_event(&self, event: &NSEvent) {
                cef::application_mac::scoped_sending_event::<SimpleApplication, _>(|| {
                    let _: () = unsafe { msg_send![super(self), sendEvent: event] };
                });
            }

            #[unsafe(method(terminate:))]
            fn terminate(&self, _sender: &AnyObject) {
                let delegate = self.delegate().unwrap();
                let delegate: &AnyObject = delegate.as_ref();
                let delegate: &SimpleAppDelegate = delegate.downcast_ref().unwrap();

                delegate.try_to_terminate_application(self);
            }
        }
    );

    /// Instance variables of `SimpleAppDelegateIvars`.
    pub struct SimpleAppDelegateIvars {
        simple_handler: Arc<Mutex<SimpleHandler>>,
    }

    define_class!(
        /// Receives notifications from the application.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[ivars = SimpleAppDelegateIvars]
        pub struct SimpleAppDelegate;

        unsafe impl NSObjectProtocol for SimpleAppDelegate {}

        impl SimpleAppDelegate {
            #[unsafe(method(createApplication:))]
            fn __create_application(&self, _app: &NSApplication) {
                let mtm = MainThreadMarker::new().unwrap();

                unsafe {
                    let _: Bool = msg_send![
                        &NSBundle::mainBundle(),
                        loadNibNamed: ns_string!("MainMenu"),
                        owner: &*NSApp(mtm),
                        topLevelObjects: std::ptr::null_mut::<*const AnyObject>()
                    ];
                };
            }

            #[unsafe(method(tryToTerminateApplication:))]
            fn __try_to_terminate_application(&self, _app: &NSApplication) {
                let handler = self.ivars().simple_handler.lock().unwrap();

                if !handler.is_closing {
                    handler.close_all_browsers(false);
                }
            }
        }

        #[allow(non_snake_case)]
        unsafe impl NSUserInterfaceValidations for SimpleAppDelegate {
            #[unsafe(method(validateUserInterfaceItem:))]
            fn validateUserInterfaceItem(
                &self,
                item: &ProtocolObject<dyn NSValidatedUserInterfaceItem>,
            ) -> bool {
                const IDC_FIND: u32 = 37000;

                item.tag() as u32 == IDC_FIND
            }
        }

        #[allow(non_snake_case)]
        unsafe impl NSApplicationDelegate for SimpleAppDelegate {
            #[unsafe(method(applicationShouldTerminate:))]
            fn applicationShouldTerminate(
                &self,
                _sender: &NSApplication,
            ) -> NSApplicationTerminateReply {
                NSApplicationTerminateReply::TerminateNow
            }

            #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
            fn applicationShouldHandleReopen_hasVisibleWindows(
                &self,
                _sender: &NSApplication,
                _has_visible_windows: bool,
            ) -> bool {
                let mut handler = self.ivars().simple_handler.lock().unwrap();

                if handler.is_closing {
                    handler.show_main_window();
                };

                false
            }

            #[unsafe(method(applicationSupportsSecureRestorableState:))]
            fn applicationSupportsSecureRestorableState(&self, _app: &NSApplication) -> bool {
                true
            }
        }
    );

    impl SimpleAppDelegate {
        pub fn new(
            mtm: objc2::MainThreadMarker,
            simple_handler: Arc<Mutex<SimpleHandler>>,
        ) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(SimpleAppDelegateIvars { simple_handler });

            unsafe { msg_send![super(this), init] }
        }

        extern_methods!(
            #[unsafe(method(createApplication:))]
            fn create_application(&self, app: &NSApplication);

            #[unsafe(method(tryToTerminateApplication:))]
            fn try_to_terminate_application(&self, app: &NSApplication);
        );
    }
}
