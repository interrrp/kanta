use std::{
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::bail;
use symphonia::{
    core::{
        io::MediaSourceStream,
        meta::{MetadataOptions, StandardTagKey, Tag},
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
}

impl TryFrom<PathBuf> for Track {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> anyhow::Result<Track> {
        let file = File::open(&path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let hint = Hint::new();
        let mut probed = get_probe()
            .format(&hint, mss, &Default::default(), &MetadataOptions::default())
            .unwrap();
        let mut lyrics: Option<String> = None;
        let metadata = probed.format.metadata();
        let Some(rev) = metadata.current() else {
            bail!("No metadata")
        };
        let tags = rev.tags();

        if let Some(lyric_tag) = find_tag(tags, StandardTagKey::Lyrics) {
            lyrics = Some(lyric_tag);
        }

        Ok(Track {
            path,
            title: find_tag(tags, StandardTagKey::TrackTitle),
            album: find_tag(tags, StandardTagKey::Album),
            artist: find_tag(tags, StandardTagKey::Artist),
            lyrics,
        })
    }
}

fn find_tag(tags: &[Tag], key: StandardTagKey) -> Option<String> {
    tags.iter()
        .find(|t| t.std_key == Some(key))
        .map(|t| t.value.to_string())
}
