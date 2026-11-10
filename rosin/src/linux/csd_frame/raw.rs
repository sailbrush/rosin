use std::ffi::CString;
use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_long;
use std::ffi::c_void;
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::os::fd::FromRawFd;
use std::os::fd::OwnedFd;
use std::ptr;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use wayland_client::Proxy;
use wayland_client::WEnum;
use wayland_client::backend::ObjectData;
use wayland_client::protocol::wl_buffer;
use wayland_client::protocol::wl_shm;
use wayland_client::protocol::wl_shm_pool;

use std::os::fd::AsFd;
use std::sync::atomic::{AtomicUsize, Ordering};
#[derive(Debug)]
pub enum GlobalError {
    MissingGlobal(&'static str),

    InvalidVersion { name: &'static str, required: u32, available: u32 },
}

const O_RWDR: i32 = 00000002;
const O_CREAT: i32 = 00000100;
const O_EXCL: i32 = 00000200;
const S_IWUSR: u32 = 0000200;
const S_IRUSR: u32 = 0000400;
const _SC_PAGESIZE: c_int = 30;
const MAP_POPULATE: c_int = 0x08000;
const MAP_NORESERVE: c_int = 0x04000;
const MAP_SHARED: c_int = 0x0001;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
unsafe extern "C" {
    pub fn shm_open(name: *const c_char, oflag: c_int, mode: u32) -> c_int;
    pub fn shm_unlink(name: *const c_char) -> c_int;
    pub fn mmap64(addr: *mut c_void, len: usize, prot: c_int, flags: c_int, fd: c_int, offset: i64) -> *mut c_void;
    pub fn sysconf(name: c_int) -> c_long;
}

#[derive(Debug)]
pub enum CreatePoolError {
    Global(GlobalError),

    Create(io::Error),
}
#[derive(Debug)]
pub(crate) struct RawPool {
    len: usize,
    pool: wl_shm_pool::WlShmPool,
    mem_file: File,
    mmap: *mut u8,
}
#[derive(Debug)]
struct ShmPoolData;

impl ObjectData for ShmPoolData {
    fn event(
        self: Arc<Self>,
        _: &wayland_client::backend::Backend,
        _: wayland_client::backend::protocol::Message<wayland_client::backend::ObjectId, OwnedFd>,
    ) -> Option<Arc<dyn ObjectData + 'static>> {
        unreachable!("wl_shm_pool has no events")
    }

    fn destroyed(&self, _: wayland_client::backend::ObjectId) {}
}
impl RawPool {
    pub fn new(shm: &wl_shm::WlShm, len: usize) -> Result<Self, CreatePoolError> {
        let mem_file = File::from(RawPool::create_raw_fd().expect("msg"));
        let pool = shm
            .send_constructor(
                wl_shm::Request::CreatePool {
                    fd: mem_file.as_fd(),
                    size: len as i32,
                },
                Arc::new(ShmPoolData),
            )
            .unwrap_or_else(|_| Proxy::inert(shm.backend().clone()));
        let mut retval = RawPool {
            len,
            pool,
            mem_file,
            mmap: std::ptr::null_mut(),
        };
        retval.create_mmap();
        Ok(retval)
    }

    fn create_raw_fd() -> io::Result<OwnedFd> {
        let time = SystemTime::now();
        let mem_file_handle = CString::new(format!("/rosin-app-{}", time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()).as_bytes()).expect("msg");
        unsafe {
            let rawfd = OwnedFd::from_raw_fd(shm_open(mem_file_handle.as_ptr(), O_CREAT | O_EXCL | O_RWDR, S_IRUSR | S_IWUSR));

            shm_unlink(mem_file_handle.as_ptr());
            Ok(rawfd)
        }
    }

    pub fn mmap(&mut self) -> &mut [u8] {
        let _retval = self.mmap;
        unsafe { std::slice::from_raw_parts_mut(self.mmap, self.len) }
    }
    fn create_mmap(&mut self) {
        let alignment = 0 % page_size() as u64;
        let aligned_offset = 0 - alignment;
        let _desc = self.mem_file.as_raw_fd();
        let map_len = self.len;
        let _map_offset = alignment;

        unsafe {
            self.mmap = mmap64(
                ptr::null_mut(),
                map_len,
                PROT_READ | PROT_WRITE,
                MAP_SHARED | MAP_POPULATE | MAP_NORESERVE,
                self.mem_file.as_raw_fd(),
                aligned_offset as i64,
            ) as *mut u8;
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn resize(&mut self, new: usize) -> io::Result<()> {
        self.len = new;
        self.mem_file.set_len(new as u64)?;
        self.pool.resize(new as i32);
        self.create_mmap();
        Ok(())
    }

    pub fn create_buffer_raw(
        &mut self,
        offset: i32,
        width: i32,
        height: i32,
        stride: i32,
        format: wl_shm::Format,
        data: Arc<dyn ObjectData + 'static>,
    ) -> wl_buffer::WlBuffer {
        self.pool
            .send_constructor(
                wl_shm_pool::Request::CreateBuffer {
                    offset,
                    width,
                    height,
                    stride,
                    format: WEnum::Value(format),
                },
                data,
            )
            .unwrap_or_else(|_| Proxy::inert(self.pool.backend().clone()))
    }
}
fn page_size() -> usize {
    static PAGE_SIZE: AtomicUsize = AtomicUsize::new(0);

    match PAGE_SIZE.load(Ordering::Relaxed) {
        0 => {
            let page_size = unsafe { sysconf(_SC_PAGESIZE) as usize };

            PAGE_SIZE.store(page_size, Ordering::Relaxed);

            page_size
        }
        page_size => page_size,
    }
}
