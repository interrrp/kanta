use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    time::Duration,
};

use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};

use crate::track::Track;

pub struct Player {
    #[allow(dead_code)] // stream needs to live
    stream: OutputStream,
    sink: Sink,
    playlist: Vec<Track>,
    playlist_index: Option<usize>,
}

impl Player {
    pub fn try_new() -> anyhow::Result<Player> {
        let stream = OutputStreamBuilder::open_default_stream()?;
        let sink = Sink::connect_new(stream.mixer());

        Ok(Player {
            stream,
            sink,
            playlist: vec![],
            playlist_index: None,
        })
    }

    pub fn jump_to_track_at(&mut self, index: usize) {
        self.playlist_index = Some(index);
        self.update_sink_to_current_track();
    }

    pub fn jump_to_previous_track(&mut self) {
        if self.playlist.is_empty() {
            return;
        }
        let Some(index) = self.playlist_index.as_mut() else {
            return;
        };
        if *index > 0 {
            *index -= 1;
        }
        self.update_sink_to_current_track();
    }

    pub fn jump_to_next_track(&mut self) {
        if self.playlist.is_empty() {
            return;
        }

        self.playlist_index = match self.playlist_index {
            // Do nothing if this is the last song in playlist
            Some(index) if index == self.playlist.len() - 1 => Some(index),
            Some(index) => Some(index + 1),
            None => Some(0),
        };

        self.update_sink_to_current_track();
    }

    pub fn play(&mut self) {
        self.sink.play();
    }

    pub fn pause(&mut self) {
        self.sink.pause();
    }

    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    pub fn is_idle(&self) -> bool {
        self.sink.empty()
    }

    pub fn playlist(&self) -> &[Track] {
        &self.playlist
    }

    pub fn playlist_index(&self) -> Option<usize> {
        self.playlist_index
    }

    pub fn add_to_playlist(&mut self, track: Track) {
        self.playlist.push(track);
    }

    pub fn load_m3u8_playlist(&mut self, path: &Path) -> anyhow::Result<()> {
        self.playlist.clear();

        let contents = fs::read_to_string(path)?;
        let tracks = contents
            .lines()
            .map(|line| Track::load(PathBuf::from(line)).unwrap());
        self.playlist.extend(tracks);

        Ok(())
    }

    pub fn export_m3u8_playlist(&mut self, path: &Path) -> anyhow::Result<()> {
        let m3u8_data = self
            .playlist
            .iter()
            .map(|track| track.path().to_str().unwrap().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(path, m3u8_data)?;
        Ok(())
    }

    pub fn clear_playlist(&mut self) {
        self.playlist.clear();
        self.update_sink_to_current_track();
    }

    pub fn position(&self) -> Duration {
        self.sink.get_pos()
    }

    pub fn set_position(&mut self, position: Duration) {
        // Ignoring the error for now
        let _ = self.sink.try_seek(position);
    }

    pub fn volume(&self) -> f32 {
        self.sink.volume()
    }

    pub fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume);
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.playlist_index
            .and_then(|position| self.playlist.get(position))
    }

    fn update_sink_to_current_track(&mut self) {
        if !self.sink.empty() {
            self.sink.skip_one();
        }

        let Some(track) = self.current_track() else {
            return;
        };

        let bytes = fs::read(track.path()).unwrap();
        let bytes_len = bytes.len() as u64;

        let source = Decoder::builder()
            .with_data(Cursor::new(bytes))
            .with_byte_len(bytes_len)
            .build()
            .unwrap();

        self.sink.append(source);
    }
}
