use uvc_sys::*;

use crate::device::DeviceHandle;
use crate::error::{Error, Result};
use crate::frame::Frame;

use std::os::raw::c_void;

unsafe impl Send for StreamHandle<'_> {}
unsafe impl Sync for StreamHandle<'_> {}
#[derive(Debug)]
/// Stream handle
pub struct StreamHandle<'a> {
    pub(crate) handle: uvc_stream_ctrl_t,
    pub(crate) devh: &'a DeviceHandle<'a>,
}

unsafe impl Send for ActiveStream<'_> {}
unsafe impl Sync for ActiveStream<'_> {}
#[derive(Debug)]
/// Active stream
///
/// Dropping this stream will stop the stream
pub struct ActiveStream<'a> {
    devh: &'a crate::DeviceHandle<'a>,
    #[allow(unused)]
    cb: *mut dyn FnMut(&Frame),
}

impl ActiveStream<'_> {
    /// Stop the stream
    pub fn stop(self) {
        // Taking ownership of the stream, which drops it
    }
}

impl Drop for ActiveStream<'_> {
    fn drop(&mut self) {
        unsafe {
            uvc_stop_streaming(self.devh.devh.as_ptr());
            drop(Box::from_raw(self.cb))
        }
    }
}

unsafe extern "C" fn trampoline<F>(frame: *mut uvc_frame, userdata: *mut c_void)
where
    F: 'static + Send + FnMut(&Frame),
{
    let panic = std::panic::catch_unwind(|| {
        if frame.is_null() {
            panic!("Frame is null");
        }
        let frame = std::mem::ManuallyDrop::new(Frame::from_raw(frame));

        if userdata.is_null() {
            panic!("Userdata is null");
        }

        let func = &mut *(userdata as *mut F);

        func(&frame);
    });

    if panic.is_err() {
        eprintln!("User defined function panicked");
        std::process::abort();
    }
}

impl<'a> StreamHandle<'a> {
    /// Begin a stream, use the callback to save the frames
    ///
    /// This function is non-blocking
    pub fn start_stream<F>(&'a mut self, cb: F) -> Result<ActiveStream<'a>>
    where
        F: 'static + Send + FnMut(&Frame),
    {
        let func = Box::into_raw(Box::new(cb));

        unsafe {
            let err = uvc_start_streaming(
                self.devh.devh.as_ptr(),
                &mut self.handle,
                Some(trampoline::<F>),
                func as *mut c_void,
                0,
            )
            .into();
            if err == Error::Success {
                Ok(ActiveStream {
                    devh: self.devh,
                    cb: func,
                })
            } else {
                Err(err)
            }
        }
    }
}
