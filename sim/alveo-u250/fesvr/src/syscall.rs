use crate::{Error, Result};

#[derive(Debug)]
#[repr(u64)]
pub enum SyscallId {
    Write,
    Exit,
}

impl TryFrom<u64> for SyscallId {
    type Error = Error;
    fn try_from(value: u64) -> Result<Self> {
        match value {
            64 => Ok(Self::Write),
            93 => Ok(Self::Exit),
            _ => Err(Error::InvalidSyscallId(value)),
        }
    }
}

#[derive(Debug)]
pub struct Syscall {
    pub syscall_id: SyscallId,
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64, // max(args(syscall) for syscalls) = 3 (write)
}

impl Syscall {
    // target system (riscv) little endian?
    pub fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 32 {
            return None;
        }

        Some(Syscall {
            syscall_id: SyscallId::try_from(u64::from_le_bytes(bytes[0..8].try_into().ok()?))
                .ok()?,
            arg0: u64::from_le_bytes(bytes[8..16].try_into().ok()?),
            arg1: u64::from_le_bytes(bytes[16..24].try_into().ok()?),
            arg2: u64::from_le_bytes(bytes[24..32].try_into().ok()?),
        })
    }
}
