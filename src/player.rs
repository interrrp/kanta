use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::anyhow;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
use souvlaki::{MediaControlEvent, MediaPosition, SeekDirection};

use crate::{media_controls::KantaMediaControls, track::Track};

pub struct Player {
    #[allow(dead_code)] // stream needs to live
    stream: OutputStream,
    sink: Sink,
    playlist: Vec<Track>,
    playlist_index: Option<usize>,
    media_controls: KantaMediaControls,
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
            media_controls: KantaMediaControls::try_new()?,
        })
    }

    pub fn jump_to_track_at(&mut self, index: usize) -> anyhow::Result<()> {
        self.playlist_index = Some(index);
        self.update_sink_to_current_track()?;
        Ok(())
    }

    pub fn jump_to_previous_track(&mut self) -> anyhow::Result<()> {
        if self.playlist.is_empty() {
            return Ok(());
        }
        let Some(index) = self.playlist_index.as_mut() else {
            return Ok(());
        };
        if *index > 0 {
            *index -= 1;
        }
        self.update_sink_to_current_track()?;
        Ok(())
    }

    pub fn jump_to_next_track(&mut self) -> anyhow::Result<()> {
        if self.playlist.is_empty() {
            return Ok(());
        }

        self.playlist_index = match self.playlist_index {
            // Do nothing if this is the last song in playlist
            Some(index) if index == self.playlist.len() - 1 => Some(index),
            Some(index) => Some(index + 1),
            None => Some(0),
        };

        self.update_sink_to_current_track()?;

        Ok(())
    }

    pub fn play(&mut self) -> anyhow::Result<()> {
        self.sink.play();
        self.update_media_control_playback()?;
        Ok(())
    }

    pub fn pause(&mut self) -> anyhow::Result<()> {
        self.sink.pause();
        self.update_media_control_playback()?;
        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
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
        let contents = fs::read_to_string(path)?;
        self.playlist = contents
            .lines()
            .map(|line| Track::load(PathBuf::from(line)))
            .collect::<Result<_, _>>()?;
        Ok(())
    }

    pub fn export_m3u8_playlist(&mut self, path: &Path) -> anyhow::Result<()> {
        let m3u8_data = self
            .playlist
            .iter()
            .map(|track| {
                track
                    .path()
                    .to_str()
                    .ok_or_else(|| anyhow!("path contains invalid UTF-8"))
                    .map(|s| s.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");
        fs::write(path, m3u8_data)?;
        Ok(())
    }

    pub fn clear_playlist(&mut self) -> anyhow::Result<()> {
        self.playlist.clear();
        self.update_sink_to_current_track()?;
        Ok(())
    }

    pub fn position(&self) -> Duration {
        self.sink.get_pos()
    }

    pub fn set_position(&mut self, position: Duration) -> anyhow::Result<()> {
        // Ignoring the error for now
        let _ = self.sink.try_seek(position);
        self.update_media_control_playback()?;
        Ok(())
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

    pub fn tick(&mut self) -> anyhow::Result<()> {
        if self.sink.empty() {
            self.jump_to_next_track()?;
        }

        while let Some(event) = self.media_controls.receive_event() {
            use MediaControlEvent::*;
            use SeekDirection::*;
            match event {
                Play => self.play()?,
                Pause => self.pause()?,
                Next => self.jump_to_next_track()?,
                Previous => self.jump_to_previous_track()?,
                SetVolume(volume) => self.set_volume(volume as f32),
                SetPosition(MediaPosition(position)) => self.set_position(position)?,
                Seek(direction) => match direction {
                    Forward => self.set_position(self.position() + Duration::from_secs(10))?,
                    Backward => self.set_position(self.position() - Duration::from_secs(10))?,
                },
                SeekBy(direction, amount) => match direction {
                    Forward => self.set_position(self.position() + amount)?,
                    Backward => self.set_position(self.position() - amount)?,
                },
                _ => eprintln!("unhandled media control event: {:?}", event),
            }
        }

        Ok(())
    }

    fn update_sink_to_current_track(&mut self) -> anyhow::Result<()> {
        if !self.sink.empty() {
            self.sink.skip_one();
        }

        let Some(track) = self.current_track().cloned() else {
            return Ok(());
        };

        let bytes = fs::read(track.path())?;
        let bytes_len = bytes.len() as u64;

        let source = Decoder::builder()
            .with_data(Cursor::new(bytes))
            .with_byte_len(bytes_len)
            .build()?;

        self.sink.append(source);

        self.media_controls.update_metadata(&track)?;
        self.update_media_control_playback()?;

        Ok(())
    }

    fn update_media_control_playback(&mut self) -> anyhow::Result<()> {
        self.media_controls
            .update_playback(self.sink.empty(), self.is_paused(), self.position())
    }
}
