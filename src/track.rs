use std::{fs::File, io::BufReader, path::Path};

use rodio::{Decoder, Source, source::Buffered};
use symphonia::{
    core::{
        io::MediaSourceStream,
        meta::{MetadataOptions, StandardTagKey},
        probe::Hint,
    },
    default::get_probe,
};

type TrackSource = Buffered<Decoder<BufReader<File>>>;

pub struct Track {
    source: TrackSource,
    name: String,
    lyrics: Option<String>,
}

impl Track {
    pub fn source(&self) -> &TrackSource {
        &self.source
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn lyrics(&self) -> Option<&str> {
        self.lyrics.as_deref()
    }
}

impl TryFrom<&Path> for Track {
    type Error = anyhow::Error;

    fn try_from(path: &Path) -> anyhow::Result<Track> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let source = Decoder::try_from(reader)?.buffered();

        let file = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let hint = Hint::new();
        let mut probed = get_probe()
            .format(&hint, mss, &Default::default(), &MetadataOptions::default())
            .unwrap();
        let mut lyrics: Option<String> = None;
        if let Some(rev) = probed.format.metadata().current()
            && let Some(lyric_tag) = rev
                .tags()
                .iter()
                .find(|t| t.std_key == Some(StandardTagKey::Lyrics))
                .map(|t| t.value.to_string())
        {
            lyrics = Some(lyric_tag);
        }

        let name = path.file_name().unwrap().to_string_lossy().to_string();

        Ok(Track {
            source,
            name,
            lyrics,
        })
    }
}
