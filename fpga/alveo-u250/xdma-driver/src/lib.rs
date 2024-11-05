use std::fs::*;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::FileExt;
use std::os::unix::io::AsRawFd;

pub type XDMAError = Box<dyn std::error::Error>;
pub type Addr = u64;

pub struct XDMAInterface {
    bar0_base: File,
    write_fd: File,
    read_fd: File,
}

impl XDMAInterface {
    pub fn try_new(
        pci_vendor: u16,
        pci_device: u16,
        domain: u16,
        bus: u8,
        dev: u8,
        func: u8,
    ) -> Result<Self, XDMAError> {
        let pci_dev = Self::pci_dev_fmt(domain, bus, dev, func);
        Self::fpga_pci_check_file_id(
            &format!("/sys/bus/pci/devices/{}/vendor", pci_dev),
            pci_vendor,
        )?;
        Self::fpga_pci_check_file_id(
            &format!("/sys/bus/pci/devices/{}/device", pci_dev),
            pci_device,
        )?;

        let xdma_id = Self::extract_xdma_id(&format!("/sys/bus/pci/devices/{}/xdma", pci_dev))?;
        let bar0_base = Self::extract_bar0_base(&xdma_id)?;
        let write_fd = Self::extract_xdma_write_fd(&xdma_id)?;
        let read_fd = Self::extract_xdma_read_fd(&xdma_id)?;
        Ok(XDMAInterface {
            bar0_base: bar0_base,
            write_fd: write_fd,
            read_fd: read_fd,
        })
    }

    fn pci_dev_fmt(domain: u16, bus: u8, device: u8, function: u8) -> String {
        format!("{:04x}:{:02x}:{:02x}.{:x}", domain, bus, device, function)
    }

    fn fpga_pci_check_file_id(path: &str, id: u16) -> Result<(), XDMAError> {
        if !path.is_empty() {
            println!("Opening {}", path);
        } else {
            panic!("Path cannot be null");
        }

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        let tmp_id = u32::from_str_radix(&line.trim()[2..], 16)
            .expect("Failed to parse ID as a hexadecimal value");

        assert_eq!(tmp_id, id as u32, "ID in file does not match the given ID");
        Ok(())
    }

    fn extract_xdma_id(path: &str) -> Result<u32, XDMAError> {
        if let Ok(entries) = read_dir(path) {
            for entry in entries {
                let entry = entry?;
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();

                println!("examining xdma/{}", file_name_str);

                // Check if the file name contains "xdma" and "_h2c_0"
                if file_name_str.contains("xdma") && file_name_str.contains("_h2c_0") {
                    if let Some(xdma_id) = file_name_str[4..]
                        .chars()
                        .take_while(|c| c.is_digit(10))
                        .collect::<String>()
                        .parse::<u32>()
                        .ok()
                    {
                        return Ok(xdma_id);
                    } else {
                        println!("No number found after 'xdma'");
                    }
                }
            }
        }
        return Err("XDMA ID not found".into());
    }

    fn extract_bar0_base(xdma_id: &u32) -> Result<File, XDMAError> {
        let user_file_name = format!("/dev/xdma{}_user", xdma_id);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&user_file_name)?;
        Ok(file)
    }

    fn extract_xdma_write_fd(xdma_id: &u32) -> Result<File, XDMAError> {
        let file_path = format!("/dev/xdma{}_h2c_0", xdma_id);
        let file = OpenOptions::new().write(true).open(file_path)?;
        return Ok(file);
    }

    fn extract_xdma_read_fd(xdma_id: &u32) -> Result<File, XDMAError> {
        let file_path = format!("/dev/xdma{}_c2h_0", xdma_id);
        let file = OpenOptions::new().read(true).open(file_path)?;
        return Ok(file);
    }

    fn fpga_axil_read(self: &Self, addr: Addr) -> Result<u32, XDMAError> {
        let mut read_buf = [0u8; 4];
        let _ = self.bar0_base.read_at(&mut read_buf, addr)?;
        let number = u32::from_le_bytes(read_buf);
        return Ok(number);
    }

    fn fpga_axil_write(self: &mut Self, addr: Addr, value: u32) -> Result<u32, XDMAError> {
        let bytes_written = self.bar0_base.write_at(&value.to_le_bytes(), addr)?;
        self.bar0_base.flush()?;
        return Ok(bytes_written as u32);
    }

    fn fpga_axi_write(self: &mut Self, addr: Addr, data: &Vec<u8>) -> Result<u32, XDMAError> {
        let bytes_written = unsafe {
            libc::pwrite(
                self.write_fd.as_raw_fd(),
                data.as_ptr() as *const libc::c_void,
                data.len(),
                addr as libc::off_t,
            )
        };
        if bytes_written < 0 {
            panic!("write failed, data ptr: {:?} len: {} addr: {:x}", data.as_ptr(), data.len(), addr);
        }
        return Ok(bytes_written as u32);
    }

    fn fpga_axi_read(self: &Self, addr: Addr, len: u32) -> Result<Vec<u8>, XDMAError> {
        let read_buf = vec![0u8; len as usize];
        let _ = unsafe {
            libc::pread(
                self.read_fd.as_raw_fd(),
                read_buf.as_ptr() as *mut libc::c_void,
                read_buf.len(),
                addr as libc::off_t,
            )
        };
        return Ok(read_buf);
    }

    pub fn read(self: &Self, addr: Addr) -> Result<u32, XDMAError> {
        let num = self.fpga_axil_read(addr)?;
        return Ok(num & 0xffffffff);
    }

    pub fn write(self: &mut Self, addr: Addr, value: u32) -> Result<(), XDMAError> {
        self.fpga_axil_write(addr, value)?;
        return Ok(());
    }

    pub fn pull(self: &Self, addr: Addr, len: u32) -> Result<Vec<u8>, XDMAError> {
        let res = self.fpga_axi_read(addr, len)?;
        return Ok(res);
    }

    pub fn push(self: &mut Self, addr: Addr, data: &Vec<u8>) -> Result<(), XDMAError> {
        self.fpga_axi_write(addr, data)?;
        return Ok(());
    }
}
