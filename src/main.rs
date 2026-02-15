use std::{fs::File, io::BufReader, time::Duration};

use iced::{
    Color, Element, Length, Subscription,
    alignment::Vertical,
    application, time,
    widget::{button, column, container, row, scrollable, slider, text},
};
use rfd::FileDialog;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
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
        .window_size((640, 360))
        .run()
}

struct Kanta {
    #[allow(dead_code)] // stream needs to live as long as the application
    stream: OutputStream,
    sink: Sink,
    current_track: Option<Track>,
}

struct Track {
    source: Box<dyn Source>,
    name: String,
    lyrics: Option<String>,
}

#[derive(Debug, Clone)]
enum KantaMessage {
    SelectAudioPath,
    Play,
    Pause,
    PositionChanged(f32),
    VolumeChanged(f32),
    UpdatePositionSlider,
}

impl Kanta {
    fn new() -> Kanta {
        let stream = OutputStreamBuilder::open_default_stream().unwrap();
        let sink = Sink::connect_new(stream.mixer());

        Kanta {
            stream,
            sink,
            current_track: None,
        }
    }

    fn view(&self) -> Element<'_, KantaMessage> {
        let file_row = {
            let select_audio_file_button =
                button("Select audio file").on_press(KantaMessage::SelectAudioPath);

            let track_name = match &self.current_track {
                Some(track) => &track.name,
                None => "None",
            };

            row![]
                .push(select_audio_file_button)
                .push(text(track_name))
                .align_y(Vertical::Center)
                .spacing(8)
        };

        let controls = {
            let play_pause_button = if self.sink.is_paused() {
                button("Play").on_press(KantaMessage::Play)
            } else {
                button("Pause").on_press(KantaMessage::Pause)
            };

            let position_slider = match &self.current_track {
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
                .push(play_pause_button)
                .push(text("Position"))
                .push(position_slider)
                .push(text("Volume"))
                .push(volume_slider)
                .align_y(Vertical::Center)
                .spacing(8)
        };

        let lyrics = match &self.current_track {
            Some(track) if matches!(&track.lyrics, Some(_)) => {
                if let Some(lyrics) = &track.lyrics {
                    container(scrollable(text(lyrics)).width(Length::Fill))
                } else {
                    container(text(""))
                }
            }
            _ => container(text("No lyrics available").color(Color::from_rgba(1.0, 1.0, 1.0, 0.5))),
        };

        column![]
            .push(file_row)
            .push(controls)
            .push(lyrics)
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
                let Ok(file) = File::open(&path) else { return };
                let source = Decoder::try_from(BufReader::new(file)).unwrap().buffered();
                if !self.sink.empty() && !self.sink.is_paused() {
                    self.sink.skip_one();
                }
                self.sink.append(source.clone());

                // Read lyrics
                let Ok(file) = File::open(&path) else { return };
                let mss = MediaSourceStream::new(Box::new(file), Default::default());
                let hint = Hint::new();
                let mut probed = get_probe()
                    .format(&hint, mss, &Default::default(), &MetadataOptions::default())
                    .unwrap();
                let mut lyrics: Option<String> = None;
                if let Some(rev) = probed.format.metadata().current() {
                    if let Some(lyric_tag) = rev
                        .tags()
                        .iter()
                        .find(|t| t.std_key == Some(StandardTagKey::Lyrics))
                        .map(|t| t.value.to_string())
                    {
                        lyrics = Some(lyric_tag);
                    }
                }

                let name = path.file_name().unwrap().to_string_lossy().to_string();

                self.current_track = Some(Track {
                    source: Box::new(source),
                    name,
                    lyrics,
                });
            }

            Play => self.sink.play(),
            Pause => self.sink.pause(),

            PositionChanged(x) => match &self.current_track {
                Some(track) => {
                    let total = track.source.total_duration().unwrap().as_secs_f32();
                    let duration = Duration::from_secs_f32(total * x);
                    let _ = self.sink.try_seek(duration);
                }
                None => {}
            },

            VolumeChanged(volume) => self.sink.set_volume(volume),

            // Tells Iced to rerender UI elements, especially the position slider
            UpdatePositionSlider => {}
        }
    }

    fn subscription(&self) -> Subscription<KantaMessage> {
        time::every(Duration::from_millis(500)).map(|_| KantaMessage::UpdatePositionSlider)
    }
}
