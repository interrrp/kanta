#![deny(clippy::all)]

use std::{collections::VecDeque, fs::File, io::BufReader, path::Path, time::Duration};

use iced::{
    Color, Element, Length, Padding, Pixels, Settings, Subscription,
    alignment::Vertical,
    application, time,
    widget::{button, column, container, row, scrollable, slider, text},
};
use rfd::FileDialog;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source, source::Buffered};
use symphonia::{
    core::{
        io::MediaSourceStream,
        meta::{MetadataOptions, StandardTagKey},
        probe::Hint,
    },
    default::get_probe,
};

fn main() -> iced::Result {
    application(Kanta::new, Kanta::update, Kanta::view)
        .subscription(Kanta::subscription)
        .title("Kanta")
        .window_size((1280, 720))
        .settings(Settings {
            default_text_size: Pixels(14.0),
            ..Default::default()
        })
        .run()
}

struct Kanta {
    #[allow(dead_code)] // stream needs to live as long as the application
    stream: OutputStream,
    sink: Sink,
    queue: VecDeque<Track>,
    queue_pos: Option<usize>,
}

struct Track {
    source: Buffered<Decoder<BufReader<File>>>,
    name: String,
    lyrics: Option<String>,
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

#[derive(Debug, Clone)]
enum KantaMessage {
    SelectAudioPath,
    Play,
    Pause,
    Prev,
    Next,
    Jump(usize),
    ClearQueue,
    PositionChanged(f32),
    VolumeChanged(f32),
    Tick,
}

impl Kanta {
    fn new() -> Kanta {
        let stream = OutputStreamBuilder::open_default_stream().unwrap();
        let sink = Sink::connect_new(stream.mixer());

        Kanta {
            stream,
            sink,
            queue: VecDeque::new(),
            queue_pos: None,
        }
    }

    fn view(&self) -> Element<'_, KantaMessage> {
        let controls = {
            let play_pause_button = if self.current_track().is_some() {
                if self.sink.is_paused() {
                    button("Play").on_press(KantaMessage::Play)
                } else {
                    button("Pause").on_press(KantaMessage::Pause)
                }
            } else {
                button("Nothing playing")
            };

            let prev_button = button("Prev")
                .on_press(KantaMessage::Prev)
                .style(button::secondary);

            let next_button = button("Next")
                .on_press(KantaMessage::Next)
                .style(button::secondary);

            let position_slider = match &self.current_track() {
                Some(track) => {
                    let elapsed = self.sink.get_pos().as_secs_f32();
                    let total = track.source.total_duration().unwrap().as_secs_f32();

                    slider(0.0..=1.0, elapsed / total, KantaMessage::PositionChanged).step(0.01)
                }
                None => slider(0.0..=100.0, 0.0, KantaMessage::PositionChanged).step(0.01),
            };

            let volume_slider =
                slider(0.0..=1.0, self.sink.volume(), KantaMessage::VolumeChanged).step(0.01);

            row![]
                .push(prev_button)
                .push(play_pause_button)
                .push(next_button)
                .push(text("Position"))
                .push(position_slider)
                .push(text("Volume"))
                .push(volume_slider)
                .align_y(Vertical::Center)
                .spacing(8)
        };

        let muted = Color::from_rgba(1.0, 1.0, 1.0, 0.5);

        let lyrics = match self
            .current_track()
            .as_ref()
            .and_then(|track| track.lyrics.as_ref())
        {
            Some(lyrics) => scrollable(text(lyrics)).width(Length::Fill),
            None => scrollable(text("No lyrics available").color(muted)).width(Length::Fill),
        };

        let queue_controls = {
            let add_track_button = button("Add track").on_press(KantaMessage::SelectAudioPath);

            let clear_button = button("Clear")
                .on_press(KantaMessage::ClearQueue)
                .style(button::danger);

            row![].push(add_track_button).push(clear_button).spacing(8)
        };

        let mut queue_songs = column![].spacing(8);
        for (index, track) in self.queue.iter().enumerate() {
            queue_songs = queue_songs.push(
                container(
                    button(track.name.as_str())
                        .on_press(KantaMessage::Jump(index))
                        .padding(0)
                        .style(button::text),
                )
                .padding(Padding {
                    left: if self.queue_pos == Some(index) {
                        16.0
                    } else {
                        2.0
                    },
                    top: 0.0,
                    bottom: 0.0,
                    right: 0.0,
                }),
            );
        }

        let queue = column![]
            .push(queue_controls)
            .push(scrollable(queue_songs))
            .width(Length::Fill)
            .spacing(8);

        let bottom_row = row![]
            .push(lyrics)
            .push(queue)
            .width(Length::Fill)
            .spacing(8);

        column![]
            .push(controls)
            .push(bottom_row)
            .padding(8)
            .spacing(8)
            .into()
    }

    fn update(&mut self, message: KantaMessage) {
        use KantaMessage::*;
        match message {
            SelectAudioPath => {
                let Some(path) = FileDialog::new().pick_file() else {
                    return;
                };

                let track = Track::try_from(path.as_path()).unwrap();
                self.queue.push_back(track);
            }

            Play => self.sink.play(),
            Pause => self.sink.pause(),

            Prev => self.prev(),
            Next => self.next(),
            Jump(index) => {
                self.queue_pos = Some(index);
                self.update_sink_to_current_track();
            }
            ClearQueue => {
                self.queue.clear();
                self.update_sink_to_current_track();
            }

            PositionChanged(x) => {
                if let Some(track) = self.current_track() {
                    let total = track.source.total_duration().unwrap().as_secs_f32();
                    let duration = Duration::from_secs_f32(total * x);
                    let _ = self.sink.try_seek(duration);
                }
            }

            VolumeChanged(volume) => self.sink.set_volume(volume),

            Tick => {
                if self.sink.empty() {
                    self.next();
                }
            }
        }
    }

    fn prev(&mut self) {
        if self.queue.is_empty() {
            return;
        }

        let Some(queue_pos) = self.queue_pos.as_mut() else {
            return;
        };

        if *queue_pos > 0 {
            *queue_pos -= 1;
        } else {
            return;
        }

        self.update_sink_to_current_track();
    }

    fn next(&mut self) {
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
        if self.queue.is_empty() {
            while !self.sink.is_paused() && !self.sink.empty() {
                self.sink.skip_one();
            }
            return;
        }

        if let Some(track) = self.current_track() {
            if !self.sink.is_paused() && !self.sink.empty() {
                self.sink.skip_one();
            }

            self.sink.append(track.source.clone());
        }
    }

    fn current_track(&self) -> Option<&Track> {
        match self.queue_pos {
            Some(pos) => self.queue.get(pos),
            None => None,
        }
    }

    fn subscription(&self) -> Subscription<KantaMessage> {
        time::every(Duration::from_millis(10)).map(|_| KantaMessage::Tick)
    }
}
