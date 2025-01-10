// Copyright 2024 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Large journal entries are sent over the socket with an empty payload, but with a single memfd
//! file descriptor that contains the literal journal entry data. See also
//! [JOURNAL_NATIVE_PROTOCOL].
//!
//! [JOURNAL_NATIVE_PROTOCOL]: https://systemd.io/JOURNAL_NATIVE_PROTOCOL/

use std::fs::File;
use std::io;
use std::io::Write;
use std::mem;
use std::os::fd::AsRawFd;
use std::os::fd::FromRawFd;
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixDatagram;
use std::path::Path;
use std::ptr;

// If the payload's too large for a single datagram, send it through a memfd, see
// https://systemd.io/JOURNAL_NATIVE_PROTOCOL/
pub(super) fn send_large_payload(socket: &UnixDatagram, payload: &[u8]) -> io::Result<usize> {
    // Write the whole payload to a memfd
    let mut mem = create_sealable()?;
    mem.write_all(payload)?;
    // Fully seal the memfd to signal journald that its backing data won't resize anymore
    // and so is safe to mmap.
    seal_fully(mem.as_raw_fd())?;
    send_one_fd_to(socket, mem.as_raw_fd(), super::JOURNALD_PATH)
}

fn create(flags: libc::c_uint) -> io::Result<File> {
    let fd = memfd_create_syscall(flags);
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd as RawFd) })
    }
}

/// Make the `memfd_create` syscall ourselves instead of going through `libc`;
/// `memfd_create` isn't supported on `glibc<2.27` so this allows us to
/// support old-but-still-used distros like Ubuntu 16.04, Debian Stretch,
/// RHEL 7, etc.
///
/// See: https://github.com/tokio-rs/tracing/issues/1879
fn memfd_create_syscall(flags: libc::c_uint) -> libc::c_int {
    unsafe {
        libc::syscall(
            libc::SYS_memfd_create,
            c"logforth-journald".as_ptr() as *const libc::c_char,
            flags,
        ) as libc::c_int
    }
}

fn create_sealable() -> io::Result<File> {
    create(libc::MFD_ALLOW_SEALING | libc::MFD_CLOEXEC)
}

fn seal_fully(fd: RawFd) -> io::Result<()> {
    let all_seals =
        libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_WRITE | libc::F_SEAL_SEAL;
    let result = unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, all_seals) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

const CMSG_BUFSIZE: usize = 64;

#[repr(C)]
union AlignedBuffer<T: Copy + Clone> {
    buffer: T,
    align: libc::cmsghdr,
}

fn assert_cmsg_bufsize() {
    let space_one_fd = unsafe { libc::CMSG_SPACE(mem::size_of::<RawFd>() as u32) };
    assert!(
        space_one_fd <= CMSG_BUFSIZE as u32,
        "cmsghdr buffer too small (< {}) to hold a single fd",
        space_one_fd
    );
}

fn send_one_fd_to<P: AsRef<Path>>(socket: &UnixDatagram, fd: RawFd, path: P) -> io::Result<usize> {
    assert_cmsg_bufsize();

    let mut addr: libc::sockaddr_un = unsafe { mem::zeroed() };
    let path_bytes = path.as_ref().as_os_str().as_bytes();
    // path_bytes may have at most sun_path + 1 bytes, to account for the trailing NUL byte.
    if addr.sun_path.len() <= path_bytes.len() {
        return Err(io::Error::from_raw_os_error(libc::ENAMETOOLONG));
    }

    addr.sun_family = libc::AF_UNIX as _;
    unsafe {
        ptr::copy_nonoverlapping(
            path_bytes.as_ptr(),
            addr.sun_path.as_mut_ptr() as *mut u8,
            path_bytes.len(),
        )
    };

    let mut msg: libc::msghdr = unsafe { mem::zeroed() };
    // Set the target address.
    msg.msg_name = &mut addr as *mut _ as *mut libc::c_void;
    msg.msg_namelen = mem::size_of::<libc::sockaddr_un>() as libc::socklen_t;

    // We send no data body with this message.
    msg.msg_iov = ptr::null_mut();
    msg.msg_iovlen = 0;

    // Create and fill the control message buffer with our file descriptor
    let mut cmsg_buffer = AlignedBuffer {
        buffer: [0u8; CMSG_BUFSIZE],
    };
    msg.msg_control = unsafe { cmsg_buffer.buffer.as_mut_ptr() as _ };
    msg.msg_controllen = unsafe { libc::CMSG_SPACE(mem::size_of::<RawFd>() as _) as _ };

    let cmsg: &mut libc::cmsghdr =
        unsafe { libc::CMSG_FIRSTHDR(&msg).as_mut() }.expect("Control message buffer exhausted");

    cmsg.cmsg_level = libc::SOL_SOCKET;
    cmsg.cmsg_type = libc::SCM_RIGHTS;
    cmsg.cmsg_len = unsafe { libc::CMSG_LEN(mem::size_of::<RawFd>() as _) as _ };

    unsafe { ptr::write(libc::CMSG_DATA(cmsg) as *mut RawFd, fd) };

    let result = unsafe { libc::sendmsg(socket.as_raw_fd(), &msg, libc::MSG_NOSIGNAL) };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        // sendmsg returns the number of bytes written
        Ok(result as usize)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cmsg_buffer_size_for_one_fd() {
        super::assert_cmsg_bufsize()
    }
}
