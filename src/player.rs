use std::{fs, io::Cursor, time::Duration};

use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

use crate::Track;

pub struct Player {
    #[allow(dead_code)] // stream needs to live
    stream: OutputStream,
    sink: Sink,
    current_track_duration: Option<Duration>,
    playlist: Vec<Track>,
    playlist_pos: Option<usize>,
}

impl Player {
    pub fn new() -> Player {
        let stream = OutputStreamBuilder::open_default_stream().unwrap();
        let sink = Sink::connect_new(stream.mixer());

        Player {
            stream,
            sink,
            current_track_duration: None,
            playlist: vec![],
            playlist_pos: None,
        }
    }

    pub fn playlist(&self) -> &[Track] {
        self.playlist.as_slice()
    }

    pub fn playlist_pos(&self) -> Option<usize> {
        self.playlist_pos
    }

    pub fn add_to_playlist(&mut self, track: Track) {
        self.playlist.push(track);

        // If it is the only track in the playlist, play it immediately
        if self.playlist.len() == 1 {
            self.jump_to_next_track();
        }
    }

    pub fn jump_to_track_at(&mut self, pos: usize) {
        if self.playlist.get(pos).is_none() {
            return;
        }
        self.playlist_pos = Some(pos);
        self.update_sink_to_current_track();
    }

    pub fn jump_to_previous_track(&mut self) {
        if self.playlist.is_empty() {
            return;
        }

        let Some(playlist_pos) = self.playlist_pos.as_mut() else {
            return;
        };

        if *playlist_pos > 0 {
            *playlist_pos -= 1;
        }

        self.update_sink_to_current_track();
    }

    pub fn jump_to_next_track(&mut self) {
        if self.playlist.is_empty() {
            return;
        }

        self.playlist_pos = match self.playlist_pos {
            // Do nothing if this is the last song in playlist
            Some(pos) if pos == self.playlist.len() - 1 => Some(pos),
            Some(pos) => Some(pos + 1),
            None => Some(0),
        };

        self.update_sink_to_current_track();
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

        self.current_track_duration = source.total_duration();
        self.sink.append(source);
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.playlist_pos.and_then(|pos| self.playlist.get(pos))
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

    pub fn pos(&self) -> Option<f32> {
        let elapsed = self.sink.get_pos().as_secs_f32();
        let total = self
            .current_track_duration
            .map(|duration| duration.as_secs_f32())?;
        Some(elapsed / total)
    }

    pub fn set_position(&mut self, pos: f32) {
        let Some(total) = self
            .current_track_duration
            .map(|duration| duration.as_secs_f32())
        else {
            return;
        };

        let duration = Duration::from_secs_f32(total * pos);

        let _ = self.sink.try_seek(duration);
    }

    pub fn volume(&self) -> f32 {
        self.sink.volume()
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.sink.set_volume(volume);
    }

    pub fn clear(&mut self) {
        self.playlist.clear();
        self.update_sink_to_current_track();
    }
}
