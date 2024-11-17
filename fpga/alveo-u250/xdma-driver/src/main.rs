use clap::Parser;
use xdma_driver::*;
use std::thread::sleep;
use std::time;
use rand::Rng;
use indicatif::ProgressBar;

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
    let fingerprint_addr = (3 * num_mods + 1) * 4;
    println!("reading from fingerprint addr: {:x}", simif.read(fingerprint_addr)?);
    simif.write(fingerprint_addr, 0xdeadbeaf)?;
    println!("reading from fingerprint addr: {:x}", simif.read(fingerprint_addr)?);

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

    fn is_aligned<T>(ptr: *const T, alignment: usize) -> bool {
        (ptr as usize) % alignment == 0
    }

    let addr =  0x2000;
    let dma_bytes = 64;

    let mut rng = rand::thread_rng();

    let dbg_filled = (3 * num_mods + 7) * 4;
    let dbg_empty  = (3 * num_mods + 8) * 4;
    let wbuf: Vec<u8> = (0..dma_bytes).map(|_| rng.gen_range(10..16)).collect();
// assert_eq!(is_aligned(wbuf.as_ptr(), 64), true);

    let empty_bytes = simif.read(dbg_empty)?;
    println!("empty_bytes: {}", empty_bytes);
    assert!(empty_bytes >= wbuf.len() as u32,
        "Not enough empty space: {} for write len {}", empty_bytes, wbuf.len());

    let written_bytes = simif.push(addr, &wbuf)?;
    assert!(written_bytes == wbuf.len() as u32,
        "Wbuf len: {}, written bytes: {}", wbuf.len(), written_bytes);

    let filled_bytes = simif.read(dbg_filled)?;
    println!("filled_bytes: {}", filled_bytes);
    // while true {
    //     let filled_bytes = simif.read(dbg_filled)?;
    //     if filled_bytes >= wbuf.len() as u32 {
    //         break;
    //     }
    // }

    let rbuf = simif.pull(addr, dma_bytes)?;
    assert_eq!(is_aligned(rbuf.as_ptr(), dma_bytes as usize), true);
    assert!(wbuf == rbuf, "wbuf: {:X?}\nrbuf: {:X?}", wbuf, rbuf);
    println!("Test Finished");

    return Ok(());
}
