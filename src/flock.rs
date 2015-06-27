use std::path::PathBuf;
use std::fs::File;
use std::io;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;

use lock::{self, LockKind, AccessMode, lock, unlock};

#[derive(Debug)]
pub enum Error {
    LockError(lock::Error),
    IoError(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoError(err)
    }
}

impl From<lock::Error> for Error {
    fn from(err: lock::Error) -> Self {
        Error::LockError(err)
    }
}

/// A type creating a lock file on demand.
///
/// It supports multiple reader, single writer semantics and encodes 
/// whether read or write access is required in an interface similar 
/// to the one of the [`RwLock`](http://doc.rust-lang.org/std/sync/struct.RwLock.html)
///
/// It will remove the lock file it possibly created in case a lock could be obtained.
#[derive(Debug)]
pub struct FileLock {
    path: PathBuf,
    file: Option<File>,
    mode: AccessMode
}

impl FileLock {
    pub fn new(path: PathBuf, mode: AccessMode) -> FileLock {
        FileLock {
            path: path,
            file: None,
            mode: mode,
        }
    }

    fn opened_file(&mut self) -> Result<&File, io::Error> {
        if let Some(ref file) = self.file {
            return Ok(file)
        }

        self.file = Some(try!(OpenOptions::new()
                                   .create(true)
                                   .read(self.mode == AccessMode::Read)
                                   .write(self.mode == AccessMode::Write)
                                   .open(&self.path)));
        Ok(self.file.as_ref().unwrap())
    }

    pub fn any_lock(&mut self, kind: LockKind) -> Result<(), Error> {
        Ok(try!(lock(try!(self.opened_file()).as_raw_fd(),
                     kind, 
                     self.mode.clone())))
    }

    pub fn lock(&mut self) -> Result<(), Error> {
        self.any_lock(LockKind::Blocking)
    }

    pub fn try_lock(&mut self) -> Result<(), Error> {
        self.any_lock(LockKind::NonBlocking)
    }

    pub fn unlock(&mut self) -> Result<(), Error> {
        match self.file {
            Some(ref file) => Ok(try!(unlock(file.as_raw_fd()))),
            None => Err(io::Error::new(io::ErrorKind::NotFound, 
                                      "unlock() called before lock() or try_lock()").into())
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        self.unlock().ok();
    }
}