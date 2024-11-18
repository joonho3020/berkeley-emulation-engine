use clap::Parser;
use xdma_driver::*;
use rand::Rng;
use indicatif::ProgressBar;
use simif::simif::*;
use simif::mmioif::*;
use simif::dmaif::*;

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

fn main() -> Result<(), SimIfErr> {
    let args = Args::parse();
    let mut simif = XDMAInterface::try_new(
        args.pci_vendor,
        args.pci_device,
        args.domain,
        args.bus,
        args.dev,
        args.func,
    )?;

    let mut driver = Driver::try_from_simif(Box::new(simif));

    println!("Testing MMIO fingerprint");
    let fgr_init = driver.ctrl_bridge.fingerprint.read(&mut driver.simif)?;
    println!("fgr_init: {:x}", fgr_init);

    driver.ctrl_bridge.fingerprint.write(&mut driver.simif, 0xdeadbeaf)?;
    println!("reading from fingerprint addr: {:x}", driver.ctrl_bridge.fingerprint.read(&mut driver.simif)?);

    let dma_bytes = 64;
    let mut rng = rand::thread_rng();

    println!("Testing Debug DMA Bridge");
    let iterations = 10000;
    let bar = ProgressBar::new(iterations);
    for i in 0..iterations {
        bar.inc(1);

        let mut wbuf: Vec<u8> = XDMAInterface::aligned_vec(0x1000, 0);
        wbuf.extend((0..dma_bytes).map(|_| rng.gen_range(10..16)));

        let written_bytes = driver.dbg_bridge.push(&mut driver.simif, &wbuf)?;
// println!("written_bytes: {}", written_bytes);

        let mut rbuf = vec![0u8; dma_bytes as usize];
        let read_bytes = driver.dbg_bridge.pull(&mut driver.simif, &mut rbuf)?;

        assert!(read_bytes == dma_bytes, "Read {} bytes, expected read {}", read_bytes, dma_bytes);
        assert!(wbuf == rbuf, "wbuf: {:X?}\nrbuf: {:X?}", wbuf, rbuf);
    }
    bar.finish();

    println!("Test Finished");

    return Ok(());
}
