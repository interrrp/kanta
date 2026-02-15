use std::time::Duration;

use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};

use crate::Track;

pub struct Player {
    #[allow(dead_code)] // stream needs to live
    stream: OutputStream,
    sink: Sink,
    queue: Vec<Track>,
    queue_pos: Option<usize>,
}

impl Player {
    pub fn new() -> Player {
        let stream = OutputStreamBuilder::open_default_stream().unwrap();
        let sink = Sink::connect_new(stream.mixer());

        Player {
            stream,
            sink,
            queue: vec![],
            queue_pos: None,
        }
    }

    pub fn queue(&self) -> &[Track] {
        self.queue.as_slice()
    }

    pub fn queue_pos(&self) -> Option<usize> {
        self.queue_pos
    }

    pub fn add_to_queue(&mut self, track: Track) {
        self.queue.push(track);

        // If it is the only track in the queue, play it immediately
        if self.queue.len() == 1 {
            self.next();
        }
    }

    pub fn jump(&mut self, pos: usize) {
        if self.queue.get(pos).is_none() {
            return;
        }
        self.queue_pos = Some(pos);
        self.update_sink_to_current_track();
    }

    pub fn prev(&mut self) {
        if self.queue.is_empty() {
            return;
        }

        let Some(queue_pos) = self.queue_pos.as_mut() else {
            return;
        };

        if *queue_pos > 0 {
            *queue_pos -= 1;
            self.update_sink_to_current_track();
        }
    }

    pub fn next(&mut self) {
        if self.queue.is_empty() {
            return;
        }

        self.queue_pos = match self.queue_pos {
            // Do nothing if this is the last song in queue
            Some(pos) if pos == self.queue.len() - 1 => Some(pos),
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

        self.sink.append(track.source().clone());
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.queue_pos.and_then(|pos| self.queue.get(pos))
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

    pub fn pos(&self) -> Option<f32> {
        let track = self.current_track()?;
        let elapsed = self.sink.get_pos().as_secs_f32();
        let total = track
            .source()
            .total_duration()
            .map(|duration| duration.as_secs_f32())?;
        Some(elapsed / total)
    }

    pub fn set_pos(&mut self, pos: f32) {
        let Some(track) = self.current_track() else {
            return;
        };

        let Some(total) = track
            .source()
            .total_duration()
            .map(|duration| duration.as_secs_f32())
        else {
            return;
        };

        let duration = Duration::from_secs_f32(total * pos);

        self.sink.try_seek(duration).unwrap();
    }

    pub fn volume(&self) -> f32 {
        self.sink.volume()
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.sink.set_volume(volume);
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.update_sink_to_current_track();
    }

    pub fn is_idle(&self) -> bool {
        self.sink.empty() && self.sink.is_paused()
    }
}
