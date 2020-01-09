/*
    prefault
    Copyright (c) 2019-2020 the prefault developers

    This file is part of prefault.

    Prefault is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Prefault is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with Prefault.  If not, see <http://www.gnu.org/licenses/>.
*/

use libc;
use rayon::prelude::*;
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
    m.par_iter().for_each(|mapping| {
        println!("{}", mapping.display());
        match File::open(&mapping) {
            Ok(f) => {
                let fd = f.into_raw_fd();

                let mut stat: libc::stat = unsafe { std::mem::zeroed() };
                let result = unsafe { libc::fstat(fd, &mut stat) };
                if result != 0 {
                    unsafe {
                        let f = ffi::CString::new("fstat").unwrap();
                        libc::perror(f.as_ptr());
                    }
                }

                unsafe {
                    libc::close(fd);
                }
            }

            Err(e) => println!("{}: {}", mapping.display(), e),
        }
    })
}

pub fn prefault_file_mappings(m: &[PathBuf]) -> io::Result<()> {
    m.par_iter().for_each(|mapping| {
        println!("{}", mapping.display());

        match File::open(&mapping) {
            Ok(f) => {
                let fd = f.into_raw_fd();

                let result = unsafe { libc::readahead(fd, 0, MAX_READAHEAD) };
                if result != 0 {
                    unsafe {
                        let f = ffi::CString::new("readahead").unwrap();
                        libc::perror(f.as_ptr());
                    }
                }

                let mut stat: libc::stat = unsafe { std::mem::zeroed() };
                let result = unsafe { libc::fstat(fd, &mut stat) };
                if result != 0 {
                    unsafe {
                        let f = ffi::CString::new("fstat").unwrap();
                        libc::perror(f.as_ptr());
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
                if addr.is_null() {
                    unsafe {
                        let f = ffi::CString::new("mmap").unwrap();
                        libc::perror(f.as_ptr());
                    }
                }

                let result =
                    unsafe { libc::madvise(addr, stat.st_size as usize, libc::MADV_WILLNEED) };
                if result != 0 {
                    unsafe {
                        let f = ffi::CString::new("madvise").unwrap();
                        libc::perror(f.as_ptr());
                    }
                }

                unsafe {
                    libc::close(fd);
                }
            }

            Err(e) => println!("{}: {}", mapping.display(), e),
        }
    });

    Ok(())
}

pub fn mlock_file_mappings(m: &[PathBuf], opts: &Options) -> io::Result<()> {
    m.par_iter().for_each(|mapping| {
        if opts.verbosity > 1 {
            println!("{}", mapping.display());
        }

        match File::open(&mapping) {
            Ok(f) => {
                let fd = f.into_raw_fd();

                let result = unsafe { libc::readahead(fd, 0, MAX_READAHEAD) };
                if result != 0 {
                    unsafe {
                        let f = ffi::CString::new("readahead").unwrap();
                        libc::perror(f.as_ptr());
                    }
                }

                let mut stat: libc::stat = unsafe { std::mem::zeroed() };
                let result = unsafe { libc::fstat(fd, &mut stat) };
                if result != 0 {
                    unsafe {
                        let f = ffi::CString::new("fstat").unwrap();
                        libc::perror(f.as_ptr());
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
                if addr.is_null() {
                    unsafe {
                        let f = ffi::CString::new("mmap").unwrap();
                        libc::perror(f.as_ptr());
                    }
                }

                let result = unsafe { libc::mlock(addr, stat.st_size as usize) };
                if result != 0 {
                    unsafe {
                        let f = ffi::CString::new("mlock").unwrap();
                        libc::perror(f.as_ptr());
                    }
                }

                unsafe {
                    libc::close(fd);
                }
            }

            Err(e) => println!("{}: {}", mapping.display(), e),
        }
    });

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
                        let f = ffi::CString::new("fstat").unwrap();
                        libc::perror(f.as_ptr());
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
                if addr.is_null() {
                    unsafe {
                        let f = ffi::CString::new("mmap").unwrap();
                        libc::perror(f.as_ptr());
                    }
                }

                let mut pages = Vec::with_capacity(((stat.st_size + 4096) / 4096) as usize);
                let result =
                    unsafe { libc::mincore(addr, stat.st_size as usize, pages.as_mut_ptr()) };
                if result != 0 {
                    unsafe {
                        let f = ffi::CString::new("mincore").unwrap();
                        libc::perror(f.as_ptr());
                    }
                } else {
                    unsafe {
                        pages.set_len(((stat.st_size + 4096) / 4096) as usize);
                    }
                }

                let result = unsafe { libc::munmap(addr, stat.st_size as usize) };
                if result != 0 {
                    unsafe {
                        let f = ffi::CString::new("munmap").unwrap();
                        libc::perror(f.as_ptr());
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
                    util::format_file_size(stat.st_size as u64),
                );

                unsafe {
                    libc::close(fd);
                }
            }

            Err(e) => println!("{}: {}", mapping.display(), e),
        }
    }

    Ok(())
}
