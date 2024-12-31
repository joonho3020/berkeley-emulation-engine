use crate::syscall::Syscall;
use std::io;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("target attempted invalid syscall id: {0}")]
    InvalidSyscallId(u64),

    #[error("target attempted syscall with invalid param: arg{arg_no}={value}")]
    InvalidSyscallArg { arg_no: u8, value: u64 },

    #[error("syscall failed on host")]
    SyscallFailed {
        io_error: io::Error,
        syscall: Syscall,
    },

    #[error("ELF parsing failed")]
    ElfError(#[from] object::Error),

    #[error("host I/O error")]
    IoError(#[from] io::Error),

    #[error("misc")]
    Misc,
}

pub type Result<T> = std::result::Result<T, Error>;
