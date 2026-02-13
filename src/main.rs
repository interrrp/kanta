use std::{env::args, fs::File, io::BufReader, process::exit};

use anyhow::Context;
use rodio::{OutputStreamBuilder, play};

fn main() -> anyhow::Result<()> {
    let stream_handle = OutputStreamBuilder::open_default_stream()?;
    let mixer = stream_handle.mixer();

    let file = File::open(args().skip(1).next().unwrap_or_else(|| {
        eprintln!("usage: kanta <audio file path>");
        exit(1);
    }))
    .context("failed to open audio file")?;

    let sink = play(mixer, BufReader::new(file))?;
    sink.sleep_until_end();

    Ok(())
}
