use crate::{elf::RiscvElf, syscall::SyscallId, Error, Result, Syscall};
use log::info;
use object::{elf::SHT_PROGBITS, read::elf::SectionHeader as _};
use std::{
    fs::{self, File},
    io::Write,
    os::fd::FromRawFd as _,
    path::Path,
};

pub trait Htif {
    fn read(&mut self, ptr: u64, buf: &mut [u8]) -> Result<()>;
    fn write(&mut self, ptr: u64, buf: &[u8]) -> Result<()>;
}

pub struct Frontend {
    elf: RiscvElf,
    to_host: u64, // pointers
    from_host: Option<u64>,
}

impl std::fmt::Debug for Frontend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frontend")
            .field("to_host", &self.to_host)
            .field("from_host", &self.from_host)
            .finish()
    }
}

impl Frontend {
    const MSIP_BASE: u64 = 0x2000000;
    const CHUNK_SIZE_BYTES: u64 = 1024;

    pub fn try_new(elf_path: impl AsRef<Path>) -> Result<Self> {
        let elf_data = fs::read(elf_path)?; // add error ctxt later
        let elf = RiscvElf::try_new(elf_data)?;
        let (to_host, from_host) = elf.extract_htif_addresses();

        Ok(Self {
            elf,
            to_host,
            from_host
        })
    }

    pub fn reset<H: Htif>(&self, htif: &mut H) -> Result<()> {
        htif.write(Self::MSIP_BASE, &[1])?;
        Ok(())
    }

    // write appropriate sections of elf into memory
    pub fn write_elf<H: Htif>(&self, htif: &mut H) -> Result<()> {
        let e = self.elf.endianness();

        for section in self.elf.sections()?.iter() {
            if section.sh_type(e) == SHT_PROGBITS && section.sh_addr(e) > 0 {
                let data = section.data(e, &*self.elf.data)?;

                let data_chunks = data.chunks(Self::CHUNK_SIZE_BYTES as usize);
                let mut addr = section.sh_addr(e) as u64;
                for chunk in data_chunks {
                    htif.write(addr, &chunk)?;
                    addr += chunk.len() as u64;
                }
            }
        }

        Ok(())
    }

    pub fn process<H: Htif>(&mut self, htif: &mut H) -> Result<bool> {
        let mut buf = [0; size_of::<u64>()];
        htif.read(self.to_host, &mut buf)?;
        let tohost = u64::from_le_bytes(buf);
        // todo: implement all of https://github.com/riscv-software-src/riscv-isa-sim/issues/364#issuecomment-607657754
        match tohost {
            1 => Ok(true),
            0 => Ok(false),
            a => {
                println!("{}", a);

                htif.write(self.to_host, &[0; size_of::<u64>()])?;

                self.dispatch_syscall(&buf, htif)?;

                println!("Entering fromhost_clear loop");

                // FIXME: Currently, instead of queueing up the fromhost requests and handling them in the
                // future, spin until the fromhost signal is cleared and write synchronously.
                // Assuming that there isn't aren't multiple syscalls in flight, this is fine.
                // Fix this later...
                'fromhost_clear: loop {
                    let mut buf = [0; size_of::<u64>()];

                    println!("htif.read from_host addr 0x{:x}", self.from_host.unwrap());

                    htif.read(self.from_host.unwrap(), &mut buf)?;
                    let fromhost = u64::from_le_bytes(buf);
                    println!("fromhost: {}", fromhost);
                    if fromhost == 0 {
                        break 'fromhost_clear;
                    }
                }
                htif.write(self.from_host.unwrap(), &[1])?;
                Ok(true)
            }
        }
    }

    fn dispatch_syscall<H: Htif>(&mut self, tohost: &[u8], htif: &mut H) -> Result<()> {
        let addr = u64::from_le_bytes(tohost[0..8].try_into().unwrap());
        let mut magicmem = [0u8; 64];
        htif.read(addr, &mut magicmem)?;

        let sc_opt = Syscall::from_le_bytes(&magicmem);
        println!("dispatch syscall, sc_opt {:?}", sc_opt);

        match sc_opt {
            Some(sc) => {
                let rc = self.execute_syscall(sc, htif)?;
                println!("execute syscall done rc: {}", rc);
                magicmem[0..8].copy_from_slice(&rc.to_le_bytes());
                println!("calling htif write magicmem: {:X?}", magicmem);
                htif.write(addr, &mut magicmem)?;
                Ok(())
            }
            _ => {
                Err(Error::Misc)
            }
        }
    }

    // execute syscall on host
    fn execute_syscall<H: Htif>(&mut self, syscall: Syscall, htif: &mut H) -> Result<u64> {
        println!("execute syscall");
        match syscall.syscall_id {
            SyscallId::Exit => {
                info!("target requested exit, exiting...");
                std::process::exit(0);
            }
            SyscallId::Write => {
                let (fd, ptr, len) = (syscall.arg0, syscall.arg1, syscall.arg2);

                let mut buf = vec![0; len as usize];
                htif.read(ptr, &mut buf)?;
                println!("buf: {:X?}", buf);

// let fd = fd.try_into().map_err(|_| Error::InvalidSyscallArg {
// arg_no: 0,
// value: syscall.arg0,
// })?;
// let mut f = unsafe { File::from_raw_fd(fd) };

// match f.write_all(&buf) {
// Ok(_) => {
// println!("write_all Ok, len {}", len);
// Ok(len)
// }
// Err(io_error) => {
// println!("write_all Err");
// Err(Error::SyscallFailed { io_error, syscall })
// }
// }
                return Ok(len);
            }
        }
    }
}
