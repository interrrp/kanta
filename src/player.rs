use std::{
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::anyhow;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
use souvlaki::{MediaControlEvent, MediaPosition, SeekDirection};

use crate::{media_controls::KantaMediaControls, track::Track};

#[derive(Default)]
pub struct Player {
    #[allow(dead_code)]
    stream: Option<OutputStream>,
    sink: Option<Sink>,
    playlist: Vec<Track>,
    playlist_index: Option<usize>,
    media_controls: Option<KantaMediaControls>,
}

impl Player {
    pub fn try_new() -> anyhow::Result<Player> {
        let stream = OutputStreamBuilder::open_default_stream()?;
        let sink = Sink::connect_new(stream.mixer());

        Ok(Player {
            stream: Some(stream),
            sink: Some(sink),
            playlist: vec![],
            playlist_index: None,
            media_controls: Some(KantaMediaControls::try_new()?),
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
            Some(index) if index == self.playlist.len() - 1 => Some(index),
            Some(index) => Some(index + 1),
            None => Some(0),
        };

        self.update_sink_to_current_track()?;

        Ok(())
    }

    pub fn play(&mut self) -> anyhow::Result<()> {
        if let Some(sink) = &self.sink {
            sink.play();
            self.update_media_control_playback()?;
        }
        Ok(())
    }

    pub fn pause(&mut self) -> anyhow::Result<()> {
        if let Some(sink) = &self.sink {
            sink.pause();
            self.update_media_control_playback()?;
        }
        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        self.sink.as_ref().map(|s| s.is_paused()).unwrap_or(true)
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
            .filter(|line| !line.trim().is_empty())
            .map(|line| Track::load(PathBuf::from(line)))
            .collect::<Result<_, _>>()?;
        self.update_sink_to_current_track()?;
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
        self.sink.as_ref().map(|s| s.get_pos()).unwrap_or_default()
    }

    pub fn set_position(&mut self, position: Duration) -> anyhow::Result<()> {
        if let Some(sink) = &self.sink {
            let _ = sink.try_seek(position);
        }
        self.update_media_control_playback()?;
        Ok(())
    }

    pub fn volume(&self) -> f32 {
        self.sink.as_ref().map(|s| s.volume()).unwrap_or(1.0)
    }

    pub fn set_volume(&self, volume: f32) {
        if let Some(sink) = &self.sink {
            sink.set_volume(volume);
        }
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.playlist_index
            .and_then(|position| self.playlist.get(position))
    }

    pub fn tick(&mut self) -> anyhow::Result<()> {
        let is_empty = self.sink.as_ref().map(|s| s.empty()).unwrap_or(true);
        if is_empty {
            self.jump_to_next_track()?;
        }

        let events: Vec<MediaControlEvent> =
            if let Some(media_controls) = self.media_controls.as_mut() {
                std::iter::from_fn(|| media_controls.receive_event()).collect()
            } else {
                vec![]
            };

        for event in events {
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
        if let Some(sink) = &self.sink {
            if !sink.empty() {
                sink.skip_one();
            }
        }

        let Some(track) = self.current_track().cloned() else {
            return Ok(());
        };

        let file = File::open(track.path())?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader)?;

        if let Some(sink) = &self.sink {
            sink.append(source);
        }

        if let Some(media_controls) = self.media_controls.as_mut() {
            media_controls.update_metadata(&track)?;
        }
        self.update_media_control_playback()?;

        Ok(())
    }

    fn update_media_control_playback(&mut self) -> anyhow::Result<()> {
        let is_empty = self.sink.as_ref().map(|s| s.empty()).unwrap_or(true);
        let is_paused = self.is_paused();
        let position = self.position();

        if let Some(media_controls) = self.media_controls.as_mut() {
            media_controls.update_playback(is_empty, is_paused, position)?;
        }
        Ok(())
    }
}
