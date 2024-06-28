// surfman/surfman/src/platform/unix/x11/connection.rs
//
//! A wrapper for X11 server connections (`DISPLAY` variables).

use super::device::{Device, NativeDevice};
use super::surface::NativeWidget;
use crate::connection::NativeConnection as NativeConnectionInterface;
use crate::egl;
use crate::egl::types::{EGLAttrib, EGLDisplay};
use crate::error::Error;
use crate::info::GLApi;
use crate::platform::generic::egl::device::EGL_FUNCTIONS;
use crate::platform::generic::egl::ffi::EGL_PLATFORM_X11_KHR;
use crate::platform::unix::generic::device::Adapter;

use euclid::default::Size2D;

use std::marker::PhantomData;
use std::os::raw::c_void;
use std::ptr;
use std::sync::Arc;
use x11::xlib::{Display, XCloseDisplay, XInitThreads, XLockDisplay, XOpenDisplay, XUnlockDisplay};

lazy_static! {
    static ref X_THREADS_INIT: () = {
        unsafe {
            XInitThreads();
        }
    };
}

/// A connection to the X11 display server.
#[derive(Clone)]
pub struct Connection {
    pub(crate) native_connection: Arc<NativeConnectionWrapper>,
}

unsafe impl Send for Connection {}

pub(crate) struct NativeConnectionWrapper {
    pub(crate) egl_display: EGLDisplay,
    x11_display: *mut Display,
    x11_display_is_owned: bool,
}

/// Wrapper for an X11 and EGL display.
#[derive(Clone)]
pub struct NativeConnection {
    /// The EGL display associated with that X11 display.
    ///
    /// You can obtain this with `eglGetPlatformDisplay()`.
    ///
    /// It is assumed that this EGL display is already initialized, via `eglInitialize()`.
    pub egl_display: EGLDisplay,
    /// The corresponding Xlib Display. This must be present; do not pass NULL.
    pub x11_display: *mut Display,
}
impl NativeConnectionInterface for NativeConnection {
    fn egl_display(&self) -> EGLDisplay {
        self.egl_display
    }
}

impl Drop for NativeConnectionWrapper {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            if self.x11_display_is_owned {
                XCloseDisplay(self.x11_display);
            }
            self.x11_display = ptr::null_mut();
        }
    }
}

impl Connection {
    /// Connects to the default display.
    #[inline]
    pub fn new() -> Result<Connection, Error> {
        unsafe {
            *X_THREADS_INIT;

            let x11_display = XOpenDisplay(ptr::null());
            if x11_display.is_null() {
                return Err(Error::ConnectionFailed);
            }

            let egl_display = create_egl_display(x11_display);

            Ok(Connection {
                native_connection: Arc::new(NativeConnectionWrapper {
                    x11_display,
                    x11_display_is_owned: true,
                    egl_display,
                }),
            })
        }
    }

    /// Wraps an existing X11 `Display` in a `Connection`.
    ///
    /// Important: Before calling this function, X11 must have be initialized in a thread-safe
    /// manner by using `XInitThreads()`. Otherwise, it will not be safe to use `surfman` from
    /// multiple threads.
    ///
    /// The display is not retained, as there is no way to do that in the X11 API. Therefore, it is
    /// the caller's responsibility to ensure that the display connection is not closed before this
    /// `Connection` object is disposed of.
    #[inline]
    pub unsafe fn from_native_connection(
        native_connection: NativeConnection,
    ) -> Result<Connection, Error> {
        Ok(Connection {
            native_connection: Arc::new(NativeConnectionWrapper {
                egl_display: native_connection.egl_display,
                x11_display: native_connection.x11_display,
                x11_display_is_owned: false,
            }),
        })
    }

    fn from_x11_display(x11_display: *mut Display, is_owned: bool) -> Result<Connection, Error> {
        unsafe {
            let egl_display = create_egl_display(x11_display);
            Ok(Connection {
                native_connection: Arc::new(NativeConnectionWrapper {
                    egl_display,
                    x11_display,
                    x11_display_is_owned: is_owned,
                }),
            })
        }
    }

    /// Returns the underlying native connection.
    #[inline]
    pub fn native_connection(&self) -> NativeConnection {
        NativeConnection {
            egl_display: self.native_connection.egl_display,
            x11_display: self.native_connection.x11_display,
        }
    }

    /// Returns the OpenGL API flavor that this connection supports (OpenGL or OpenGL ES).
    #[inline]
    pub fn gl_api(&self) -> GLApi {
        GLApi::GL
    }

    /// Returns the "best" adapter on this system, preferring high-performance hardware adapters.
    ///
    /// This is an alias for `Connection::create_hardware_adapter()`.
    #[inline]
    pub fn create_adapter(&self) -> Result<Adapter, Error> {
        self.create_hardware_adapter()
    }

    /// Returns the "best" adapter on this system, preferring high-performance hardware adapters.
    #[inline]
    pub fn create_hardware_adapter(&self) -> Result<Adapter, Error> {
        Ok(Adapter::hardware())
    }

    /// Returns the "best" adapter on this system, preferring low-power hardware adapters.
    #[inline]
    pub fn create_low_power_adapter(&self) -> Result<Adapter, Error> {
        Ok(Adapter::low_power())
    }

    /// Returns the "best" adapter on this system, preferring software adapters.
    #[inline]
    pub fn create_software_adapter(&self) -> Result<Adapter, Error> {
        Ok(Adapter::software())
    }

    /// Opens the hardware device corresponding to the given adapter.
    ///
    /// Device handles are local to a single thread.
    #[inline]
    pub fn create_device(&self, adapter: &Adapter) -> Result<Device, Error> {
        Device::new(self, adapter)
    }

    /// Opens the hardware device corresponding to the adapter wrapped in the given native
    /// device.
    ///
    /// This is present for compatibility with other backends.
    #[inline]
    pub unsafe fn create_device_from_native_device(
        &self,
        native_device: NativeDevice,
    ) -> Result<Device, Error> {
        Device::new(self, &native_device.adapter)
    }

    /// Opens the display connection corresponding to the given `RawDisplayHandle`.
    #[cfg(feature = "sm-raw-window-handle-05")]
    pub fn from_raw_display_handle(
        raw_handle: rwh_05::RawDisplayHandle,
    ) -> Result<Connection, Error> {
        use rwh_05::RawDisplayHandle::Xcb;
        use rwh_05::RawDisplayHandle::Xlib;
        use rwh_05::XlibDisplayHandle;
        let display = match raw_handle {
            Xlib(XlibDisplayHandle { display, .. }) => display as *mut Display,
            Xcb(_) => return Err(Error::Unimplemented),
            _ => return Err(Error::IncompatibleRawDisplayHandle),
        };

        Connection::from_x11_display(display, false)
    }

    /// Opens the display connection corresponding to the given `DisplayHandle`.
    #[cfg(feature = "sm-raw-window-handle-06")]
    pub fn from_display_handle(handle: rwh_06::DisplayHandle) -> Result<Connection, Error> {
        use rwh_06::RawDisplayHandle::Xcb;
        use rwh_06::RawDisplayHandle::Xlib;
        use rwh_06::XlibDisplayHandle;
        let display = match handle.as_raw() {
            Xlib(XlibDisplayHandle {
                display: Some(display),
                ..
            }) => display.as_ptr() as *mut Display,
            Xcb(_) => return Err(Error::Unimplemented),
            _ => return Err(Error::IncompatibleRawDisplayHandle),
        };

        Connection::from_x11_display(display, false)
    }

    /// Create a native widget from a raw pointer
    pub unsafe fn create_native_widget_from_ptr(
        &self,
        raw: *mut c_void,
        _size: Size2D<i32>,
    ) -> NativeWidget {
        NativeWidget {
            window: std::mem::transmute(raw),
        }
    }

    /// Create a native widget type from the given `RawWindowHandle`.
    #[cfg(feature = "sm-raw-window-handle-05")]
    pub fn create_native_widget_from_raw_window_handle(
        &self,
        raw_handle: rwh_05::RawWindowHandle,
        _size: Size2D<i32>,
    ) -> Result<NativeWidget, Error> {
        use rwh_05::RawWindowHandle::Xlib;

        match raw_handle {
            Xlib(handle) => Ok(NativeWidget {
                window: handle.window,
            }),
            _ => Err(Error::IncompatibleNativeWidget),
        }
    }

    /// Create a native widget type from the given `WindowHandle`.
    #[cfg(feature = "sm-raw-window-handle-06")]
    pub fn create_native_widget_from_window_handle(
        &self,
        handle: rwh_06::WindowHandle,
        _size: Size2D<i32>,
    ) -> Result<NativeWidget, Error> {
        use rwh_06::RawWindowHandle::Xlib;

        match handle.as_raw() {
            Xlib(handle) => Ok(NativeWidget {
                window: handle.window,
            }),
            _ => Err(Error::IncompatibleNativeWidget),
        }
    }
}

impl NativeConnectionWrapper {
    #[inline]
    pub(crate) fn lock_display(&self) -> DisplayGuard {
        unsafe {
            let display = self.x11_display;
            XLockDisplay(display);
            DisplayGuard {
                display,
                phantom: PhantomData,
            }
        }
    }
}

pub(crate) struct DisplayGuard<'a> {
    display: *mut Display,
    phantom: PhantomData<&'a ()>,
}

impl<'a> Drop for DisplayGuard<'a> {
    fn drop(&mut self) {
        unsafe {
            XUnlockDisplay(self.display);
        }
    }
}

impl<'a> DisplayGuard<'a> {
    #[inline]
    pub(crate) fn display(&self) -> *mut Display {
        self.display
    }
}

unsafe fn create_egl_display(display: *mut Display) -> EGLDisplay {
    EGL_FUNCTIONS.with(|egl| {
        let display_attributes = [egl::NONE as EGLAttrib];
        let egl_display = egl.GetPlatformDisplay(
            EGL_PLATFORM_X11_KHR,
            display as *mut c_void,
            display_attributes.as_ptr(),
        );

        let (mut egl_major_version, mut egl_minor_version) = (0, 0);
        let ok = egl.Initialize(egl_display, &mut egl_major_version, &mut egl_minor_version);
        assert_ne!(ok, egl::FALSE);

        egl_display
    })
}
