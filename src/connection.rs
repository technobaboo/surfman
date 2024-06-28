// surfman/surfman/src/connection.rs
//
//! The abstract interface that all connections conform to.

use crate::egl::types::EGLDisplay;
use crate::Error;
use crate::GLApi;

use euclid::default::Size2D;

use std::os::raw::c_void;

/// Methods relating to display server connections.
pub trait NativeConnection: Sized {
    /// Gets the EGLDisplay that is created from the connection.
    fn egl_display(&self) -> EGLDisplay;
}

/// Methods relating to display server connections.
pub trait Connection: Sized {
    /// The adapter type associated with this connection.
    type Adapter;
    /// The device type associated with this connection.
    type Device;
    /// The native type associated with this connection.
    type NativeConnection: NativeConnection;
    /// The native device type associated with this connection.
    type NativeDevice;
    /// The native widget type associated with this connection.
    type NativeWidget;

    /// Connects to the default display.
    fn new() -> Result<Self, Error>;

    /// Returns the native connection corresponding to this connection.
    fn native_connection(&self) -> Self::NativeConnection;

    /// Returns the OpenGL API flavor that this connection supports (OpenGL or OpenGL ES).
    fn gl_api(&self) -> GLApi;

    /// Returns the "best" adapter on this system, preferring high-performance hardware adapters.
    ///
    /// This is an alias for `Connection::create_hardware_adapter()`.
    fn create_adapter(&self) -> Result<Self::Adapter, Error>;

    /// Returns the "best" adapter on this system, preferring high-performance hardware adapters.
    fn create_hardware_adapter(&self) -> Result<Self::Adapter, Error>;

    /// Returns the "best" adapter on this system, preferring low-power hardware adapters.
    fn create_low_power_adapter(&self) -> Result<Self::Adapter, Error>;

    /// Returns the "best" adapter on this system, preferring software adapters.
    fn create_software_adapter(&self) -> Result<Self::Adapter, Error>;

    /// Opens a device.
    fn create_device(&self, adapter: &Self::Adapter) -> Result<Self::Device, Error>;

    /// Wraps an existing native device type in a device.
    unsafe fn create_device_from_native_device(
        &self,
        native_device: Self::NativeDevice,
    ) -> Result<Self::Device, Error>;

    /// Opens the display connection corresponding to the given `RawDisplayHandle`.
    #[cfg(feature = "sm-raw-window-handle-05")]
    fn from_raw_display_handle(raw_handle: rwh_05::RawDisplayHandle) -> Result<Self, Error>;

    /// Opens the display connection corresponding to the given `DisplayHandle`.
    #[cfg(feature = "sm-raw-window-handle-06")]
    fn from_display_handle(handle: rwh_06::DisplayHandle) -> Result<Self, Error>;

    /// Creates a native widget from a raw pointer
    unsafe fn create_native_widget_from_ptr(
        &self,
        raw: *mut c_void,
        size: Size2D<i32>,
    ) -> Self::NativeWidget;

    /// Create a native widget type from the given `RawWindowHandle`.
    #[cfg(feature = "sm-raw-window-handle-05")]
    fn create_native_widget_from_raw_window_handle(
        &self,
        window: rwh_05::RawWindowHandle,
        size: Size2D<i32>,
    ) -> Result<Self::NativeWidget, Error>;

    /// Create a native widget type from the given `WindowHandle`.
    #[cfg(feature = "sm-raw-window-handle-06")]
    fn create_native_widget_from_window_handle(
        &self,
        window: rwh_06::WindowHandle,
        size: Size2D<i32>,
    ) -> Result<Self::NativeWidget, Error>;
}
