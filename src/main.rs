use argh::FromArgs;

#[derive(FromArgs)]
/// dd writer goes drrrrrrrr
pub struct AppArgs {
    /// the file to read
    #[argh(option)]
    pub input: String,

    /// the file to write
    #[argh(option)]
    pub output: String,

    /// set the blocksize, default is 1MiB
    #[argh(option)]
    pub blocksize: Option<usize>,

    /// amount of blocks to write, default is to write until EOF
    #[argh(option)]
    pub count: Option<usize>,

    /// show the progress of the OS sync operation, might give invalid numbers
    #[argh(switch)]
    pub sync_progress: bool,
}

use async_std::fs::{File, OpenOptions};
use async_std::io::prelude::*;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Instant;

// const CLEAR_LINE: &str = "\r\x1b[K";

fn main() {
    async_std::task::block_on(_main()).unwrap();
}

async fn _main() -> Result<(), std::io::Error> {
    let args: AppArgs = argh::from_env();
    let block_size = args.blocksize.unwrap_or(1024 * 1024);

    let total = if let Some(count) = args.count {
        count * block_size
    } else {
        std::fs::metadata(&args.input)
            .expect("Could not read input file")
            .len() as usize
    };

    if let Ok(meta) = std::fs::metadata(&args.output) {
        if meta.is_dir() {
            println!("The output file is a directory. Aborting");
            std::process::exit(0);
        }
    }

    let source = File::open(&args.input).await?;

    let target = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&args.output)
        .await
        .expect("Could not open output file");

    let (tx, rx) = channel();

    let t = Instant::now();
    let handle = thread::spawn(move || {
        async_std::task::block_on(writer_thread(tx, block_size, args.count, source, target))
            .unwrap()
    });
    println!("Writing file to OS buffer");

    let mut last_written = 0;
    for written in rx.iter() {
        std::thread::sleep(std::time::Duration::from_millis(100));
        println!(
            "Progress {}% ({}MiB of {}MiB, {:.1}MiB/s)",
            ((written as f32 / total as f32) * 100.0).floor(),
            written / 1_000_000,
            total / 1_000_000,
            (written as f32 - last_written as f32) / 1_000_000.0
        );
        last_written = written;

        if written == total {
            println!("Syncing filesystem");
            break;
        };
    }

    let mut last_written = 0;
    loop {
        if let Ok(signal) = rx.try_recv() {
            if signal == 0 {
                let elapsed = t.elapsed();
                println!(
                    "Finished in {:?}, {:.1}MiB/s",
                    elapsed,
                    (total as f32 / elapsed.as_secs() as f32) / 1_000_000.0
                );

                return Ok(());
            }
        }

        let meminfo = async_std::fs::read_to_string("/proc/meminfo").await?;
        let line = meminfo
            .split('\n')
            .filter(|s| s.contains("Dirty"))
            .nth(0)
            .unwrap();
        let dirty = line.split(":").nth(1).unwrap().replace("kB", "");
        let dirty: usize = dirty.trim().parse().unwrap();
        let progress = total - dirty * 1000;

        println!(
            "Progress {}% ({}MiB of {}MiB, {:.1}MiB/s)",
            ((progress as f32 / total as f32) * 100.0).floor(),
            progress / 1_000_000,
            total / 1_000_000,
            (progress as f32 - last_written as f32) / 1_000_000.0
        );
        last_written = progress;
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

async fn writer_thread(
    tx: Sender<usize>,
    block_size: usize,
    count: Option<usize>,
    mut source: File,
    mut target: File,
) -> Result<(), std::io::Error> {
    let mut count = count.unwrap_or(usize::max_value());
    let mut written = 0;
    let mut buf = vec![0; block_size];
    let mut last_print = Instant::now();
    let mut read = 1;
    while read != 0 && count > 0 {
        read = source.read(&mut buf).await?;
        target.write(&mut buf).await?;

        written += read;
        count -= 1;
        if last_print.elapsed().as_millis() > 500 {
            tx.send(written).unwrap();
            last_print = Instant::now();
        }
    }
    tx.send(written).unwrap();
    target.sync_data().await.unwrap_or_default();
    tx.send(0).unwrap();

    Ok(())
}
