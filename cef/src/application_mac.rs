use objc2::{
    extern_protocol, rc::Retained, runtime::Bool, ClassType, DowncastTarget, MainThreadMarker,
};
use objc2_app_kit::NSApp;

extern_protocol!(
    /// The binding of `CrAppProtocol`.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe trait CrAppProtocol {
        #[unsafe(method(isHandlingSendEvent))]
        unsafe fn is_handling_send_event(&self) -> Bool;
    }
);

extern_protocol!(
    /// The binding of `CrAppControlProtocol`.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe trait CrAppControlProtocol: CrAppProtocol {
        #[unsafe(method(setHandlingSendEvent:))]
        unsafe fn set_handling_send_event(&self, handling_send_event: Bool);
    }
);

extern_protocol!(
    /// The binding of `CefAppProtocol`.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe trait CefAppProtocol: CrAppControlProtocol {}
);

/// It controls the state of `isHandlingSendEvent` on the object that implements [`CefAppProtocol`]
/// in the event loop to ensure that it resets properly.
///
/// It should be called during performing `-[NSApplication sendEvent:]`.
///
/// # Examples
/// ```rust
/// impl SimpleApplication {
///     #[unsafe(method(sendEvent:))]
///     fn send_event(&self, event: &NSEvent) {
///         // SimpleApplication must implements `CefAppProtocol`.
///         cef::application_mac::scoped_sending_event::<SimpleApplication, _>(|| {
///             // Perform `-[NSApplication sendEvent:]`.
///             let _: () = unsafe { msg_send![super(self), sendEvent: event] };
///         });
///     }
/// }
/// ```
///
/// # Panics
///
/// This may panic if you don't call on main thread,
/// or `NSApp` is not the type of the generics `App`.
pub fn scoped_sending_event<App, T>(f: impl FnOnce() -> T) -> T
where
    // The struct type is needed to ensure that `NSApp` implements `CefAppProtocol`.
    App: ClassType + DowncastTarget + CefAppProtocol,
{
    // This function is port of `CefScopedSendingEvent` class.

    let mtm = MainThreadMarker::new().expect("This function must be called on main thread.");
    let app: Retained<App> = NSApp(mtm)
        .downcast()
        .expect("`NSApp` is not the type of the generics `App`.");

    unsafe {
        let handling = app.is_handling_send_event();
        app.set_handling_send_event(Bool::YES);

        let result = f();
        app.set_handling_send_event(handling);

        result
    }
}
