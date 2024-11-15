use clap::Parser;
use xdma_driver::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(long, default_value_t = 0x0000)]
    pub domain: u16,

    #[arg(long, default_value_t = 0x17)]
    pub bus: u8,

    #[arg(long, default_value_t = 0x00)]
    pub dev: u8,

    #[arg(long, default_value_t = 0x0)]
    pub func: u8,

    #[arg(long, default_value_t = 0x10ee)]
    pub pci_vendor: u16,

    #[arg(long, default_value_t = 0x903f)]
    pub pci_device: u16,
}

fn main() -> Result<(), XDMAError> {
    let args = Args::parse();
    let mut simif = XDMAInterface::try_new(
        args.pci_vendor,
        args.pci_device,
        args.domain,
        args.bus,
        args.dev,
        args.func,
    )?;

    let num_mods = 9;
    let fingerprint_addr = (3 * num_mods + 6) * 4;
    println!("reading from fingerprint addr: {:x}", simif.read(fingerprint_addr)?);
// simif.write(fingerprint_addr, 0xdeadcafe)?;
// println!("reading from fingerprint addr: {:x}", simif.read(fingerprint_addr)?);

// for i in 0..64 {
// let addr = i * 4;
// simif.write(addr, 0xbabebabe)?;
// simif.write(addr, 0xcafecafe)?;
// simif.write(addr, 0xdeaddead)?;
// println!("read from {:x}: {:x}", addr, simif.read(addr)?);
// }

// for i in 0..16 {
// let addr = i * 4096;
// let wbuf = vec![0xdu8; 128];
// simif.push(addr, &wbuf)?;
// let rbuf = simif.pull(addr, 128)?;
// if wbuf != rbuf {
// println!("dma mismatch :(");
// println!("wbuf: {:?}", wbuf);
// println!("rbuf: {:?}", rbuf);
// } else {
// println!("dma match :)");
// }
// }


    return Ok(());
}
