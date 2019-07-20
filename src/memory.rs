use libc;
use std::ffi;
use std::fs::File;
use std::io;
use std::os::unix::io::IntoRawFd;
use std::path::PathBuf;
use std::ptr;

use crate::util;
use crate::Options;

const MAX_READAHEAD: usize = 10 * 1024 * 1024;

pub fn prime_dentry_cache(m: &[PathBuf]) {
    for mapping in m.iter() {
        println!("{}", mapping.display());
        match File::open(&mapping) {
            Ok(f) => {
                let fd = f.into_raw_fd();

                let mut stat: libc::stat = unsafe { std::mem::zeroed() };
                let result = unsafe { libc::fstat(fd, &mut stat) };
                if result != 0 {
                    unsafe {
                        libc::perror(ffi::CString::new("fstat").unwrap().as_ptr());
                    }
                }
            }

            Err(e) => println!("{}: {}", mapping.display(), e),
        }
    }
}

pub fn prefault_file_mappings(m: &[PathBuf]) -> io::Result<()> {
    for mapping in m.iter() {
        println!("{}", mapping.display());

        match File::open(&mapping) {
            Ok(f) => {
                let fd = f.into_raw_fd();

                let result = unsafe { libc::readahead(fd, 0, MAX_READAHEAD) };
                if result != 0 {
                    unsafe {
                        libc::perror(ffi::CString::new("readahead").unwrap().as_ptr());
                    }
                }

                let mut stat: libc::stat = unsafe { std::mem::zeroed() };
                let result = unsafe { libc::fstat(fd, &mut stat) };
                if result != 0 {
                    unsafe {
                        libc::perror(ffi::CString::new("fstat").unwrap().as_ptr());
                    }
                }

                let addr: *mut core::ffi::c_void = unsafe {
                    libc::mmap(
                        ptr::null_mut(),
                        stat.st_size as usize,
                        libc::PROT_READ,
                        libc::MAP_PRIVATE,
                        fd,
                        0,
                    )
                };
                if addr == ptr::null_mut() {
                    unsafe {
                        libc::perror(ffi::CString::new("mmap").unwrap().as_ptr());
                    }
                }

                let result =
                    unsafe { libc::madvise(addr, stat.st_size as usize, libc::MADV_WILLNEED) };
                if result != 0 {
                    unsafe {
                        libc::perror(ffi::CString::new("madvise").unwrap().as_ptr());
                    }
                }
            }

            Err(e) => println!("{}: {}", mapping.display(), e),
        }
    }

    Ok(())
}

pub fn mlock_file_mappings(m: &[PathBuf], opts: &Options) -> io::Result<()> {
    for mapping in m.iter() {
        if opts.verbosity > 1 {
            println!("{}", mapping.display());
        }

        match File::open(&mapping) {
            Ok(f) => {
                let fd = f.into_raw_fd();

                let result = unsafe { libc::readahead(fd, 0, MAX_READAHEAD) };
                if result != 0 {
                    unsafe {
                        libc::perror(ffi::CString::new("readahead").unwrap().as_ptr());
                    }
                }

                let mut stat: libc::stat = unsafe { std::mem::zeroed() };
                let result = unsafe { libc::fstat(fd, &mut stat) };
                if result != 0 {
                    unsafe {
                        libc::perror(ffi::CString::new("fstat").unwrap().as_ptr());
                    }
                }

                let addr: *mut core::ffi::c_void = unsafe {
                    libc::mmap(
                        ptr::null_mut(),
                        stat.st_size as usize,
                        libc::PROT_READ,
                        libc::MAP_PRIVATE,
                        fd,
                        0,
                    )
                };
                if addr == ptr::null_mut() {
                    unsafe {
                        libc::perror(ffi::CString::new("mmap").unwrap().as_ptr());
                    }
                }

                let result = unsafe { libc::mlock(addr, stat.st_size as usize) };
                if result != 0 {
                    unsafe {
                        libc::perror(ffi::CString::new("mlock").unwrap().as_ptr());
                    }
                }
            }

            Err(e) => println!("{}: {}", mapping.display(), e),
        }
    }

    Ok(())
}

pub fn print_fincore(m: &[PathBuf]) -> io::Result<()> {
    for mapping in m.iter() {
        match File::open(&mapping) {
            Ok(f) => {
                let fd = f.into_raw_fd();

                let mut stat: libc::stat = unsafe { std::mem::zeroed() };
                let result = unsafe { libc::fstat(fd, &mut stat) };
                if result != 0 {
                    unsafe {
                        libc::perror(ffi::CString::new("fstat").unwrap().as_ptr());
                    }
                }

                let addr: *mut core::ffi::c_void = unsafe {
                    libc::mmap(
                        ptr::null_mut(),
                        stat.st_size as usize,
                        libc::PROT_NONE,
                        libc::MAP_PRIVATE,
                        fd,
                        0,
                    )
                };
                if addr == ptr::null_mut() {
                    unsafe {
                        libc::perror(ffi::CString::new("mmap").unwrap().as_ptr());
                    }
                }

                let mut pages = Vec::with_capacity(((stat.st_size + 4096) / 4096) as usize);
                let result =
                    unsafe { libc::mincore(addr, stat.st_size as usize, pages.as_mut_ptr()) };
                if result != 0 {
                    unsafe {
                        libc::perror(ffi::CString::new("mincore").unwrap().as_ptr());
                    }
                } else {
                    unsafe {
                        pages.set_len(((stat.st_size + 4096) / 4096) as usize);
                    }
                }

                let result = unsafe { libc::munmap(addr, stat.st_size as usize) };
                if result != 0 {
                    unsafe {
                        libc::perror(ffi::CString::new("munmap").unwrap().as_ptr());
                    }
                }

                let mut page_cnt = 0;
                for page in pages.iter() {
                    if page & 0x1 != 0 {
                        page_cnt += 1;
                    }
                }

                let fincore_percentage = (page_cnt * 100) / ((stat.st_size + 4096) / 4096);
                println!(
                    "{:3}% {:5} {} ({})",
                    fincore_percentage,
                    page_cnt,
                    mapping.display(),
                    util::format_filesize(stat.st_size as u64),
                );
            }

            Err(e) => println!("{}: {}", mapping.display(), e),
        }
    }

    Ok(())
}
