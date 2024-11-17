use clap::Parser;
use xdma_driver::*;
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

    fn is_aligned<T>(ptr: *const T, alignment: usize) -> bool {
        (ptr as usize) % alignment == 0
    }

    let addr =  0x2000;
    let dma_bytes = 64;
    let dbg_filled = (3 * num_mods + 9) * 4;
    let dbg_empty  = (3 * num_mods + 10) * 4;

    let mut rng = rand::thread_rng();
    let iterations = 10000;
    let bar = ProgressBar::new(iterations);
    for i in 0..iterations {
        bar.inc(1);
        let mut wbuf: Vec<u8> = XDMAInterface::aligned_vec(0x1000, 0);
        wbuf.extend((0..dma_bytes).map(|_| rng.gen_range(10..16)));


        let empty_bytes = simif.read(dbg_empty)?;
        assert!(empty_bytes >= wbuf.len() as u32,
            "Not enough empty space: {} for write len {}", empty_bytes, wbuf.len());


        let pre_read_filled_bytes = simif.read(dbg_filled)?;
        assert!(pre_read_filled_bytes == 0, "Buffer filled before a write happend: {}", pre_read_filled_bytes);

        let written_bytes = simif.push(addr, &wbuf)?;
        assert!(written_bytes == wbuf.len() as u32,
            "Wbuf len: {}, written bytes: {}", wbuf.len(), written_bytes);

        let filled_bytes = simif.read(dbg_filled)?;
        assert!(filled_bytes == dma_bytes, "Read side didn't receive data yet, filled_bytes: {}", filled_bytes);

        let rbuf = simif.pull(addr, dma_bytes)?;
        assert_eq!(is_aligned(rbuf.as_ptr(), dma_bytes as usize), true);
        assert!(wbuf == rbuf, "wbuf: {:X?}\nrbuf: {:X?}", wbuf, rbuf);
    }
    bar.finish();

    println!("Test Finished");

    return Ok(());
}
