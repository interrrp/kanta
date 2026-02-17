use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{anyhow, bail};
use rodio::{Decoder, Source};
use symphonia::{
    core::{
        io::MediaSourceStream,
        meta::{MetadataOptions, StandardTagKey},
        probe::Hint,
    },
    default::get_probe,
};

pub struct Track {
    path: PathBuf,
    title: Option<String>,
    album: Option<String>,
    artist: Option<String>,
    lyrics: Option<String>,
    duration: Duration,
}

impl Track {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn album(&self) -> Option<&str> {
        self.album.as_deref()
    }

    pub fn artist(&self) -> Option<&str> {
        self.artist.as_deref()
    }

    pub fn lyrics(&self) -> Option<&str> {
        self.lyrics.as_deref()
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }
}

impl Track {
    pub fn load(path: PathBuf) -> anyhow::Result<Track> {
        let file = File::open(&path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let hint = Hint::new();
        let mut probed = get_probe()
            .format(&hint, mss, &Default::default(), &MetadataOptions::default())
            .unwrap();
        let metadata = probed.format.metadata();
        let Some(rev) = metadata.current() else {
            bail!("No metadata")
        };

        // Ideally we should calculate duration with Symphonia as well to avoid re-reading
        // the file, but it's much more accurate (and convenient) to use Rodio here
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader)?;
        let duration = source
            .total_duration()
            .ok_or(anyhow!("track has no total duration"))?;

        let find_tag = |key| {
            rev.tags()
                .iter()
                .find(|t| t.std_key == Some(key))
                .map(|t| t.value.to_string())
        };

        Ok(Track {
            path,
            title: find_tag(StandardTagKey::TrackTitle),
            album: find_tag(StandardTagKey::Album),
            artist: find_tag(StandardTagKey::Artist),
            lyrics: find_tag(StandardTagKey::Lyrics),
            duration,
        })
    }
}
