use std::{
    env::args,
    fs::File,
    io::BufReader,
    path::PathBuf,
    process::exit,
    time::{Duration, Instant},
};

use anyhow::Context;
use iced::{
    Element, Subscription, application, time,
    widget::{Slider, button, column, row, slider, text},
    window::frames,
};
use rfd::FileDialog;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source, play};

fn main() -> iced::Result {
    // let stream_handle = OutputStreamBuilder::open_default_stream()?;
    // let mixer = stream_handle.mixer();

    // let file = File::open(args().skip(1).next().unwrap_or_else(|| {
    //     eprintln!("usage: kanta <audio file path>");
    //     exit(1);
    // }))
    // .context("failed to open audio file")?;

    // let sink = play(mixer, BufReader::new(file))?;
    // sink.sleep_until_end();

    application(Kanta::default, Kanta::update, Kanta::view)
        .subscription(Kanta::subscription)
        .run()
}

struct Kanta {
    selected_audio_path: Option<PathBuf>,

    stream: OutputStream,
    sink: Sink,
    source: Option<Box<dyn Source>>,

    ui_volume: u8,
}

#[derive(Clone)]
enum KantaMessage {
    SelectAudioPath,
    Play,
    Pause,
    PositionChanged(f32),
    VolumeChanged(u8),
    Tick(Instant),
}

impl Default for Kanta {
    fn default() -> Kanta {
        let stream = OutputStreamBuilder::open_default_stream().unwrap();
        let sink = Sink::connect_new(stream.mixer());

        Kanta {
            selected_audio_path: None,
            stream,
            sink,
            source: None,
            ui_volume: 100,
        }
    }
}

impl Kanta {
    fn view(&self) -> Element<'_, KantaMessage> {
        let playback_slider: Slider<'_, f32, KantaMessage> = match &self.source {
            Some(src) => {
                let elapsed = self.sink.get_pos().as_secs_f32();
                let total = src.total_duration().unwrap().as_secs_f32();

                slider(0.0..=1.0, elapsed / total, KantaMessage::PositionChanged)
            }
            None => slider(0.0..=100.0, 0.0, KantaMessage::PositionChanged),
        };

        column![
            row![
                button(match &self.selected_audio_path {
                    Some(path) => path.to_str().unwrap(),
                    None => "Select audio file",
                })
                .on_press(KantaMessage::SelectAudioPath),
            ],
            row![
                if self.sink.is_paused() {
                    button("Play").on_press(KantaMessage::Play)
                } else {
                    button("Pause").on_press(KantaMessage::Pause)
                },
                text("Playback"),
                playback_slider,
                text("Volume"),
                slider(0..=100, self.ui_volume, KantaMessage::VolumeChanged),
            ],
        ]
        .into()
    }

    fn update(&mut self, message: KantaMessage) {
        use KantaMessage::*;
        match message {
            SelectAudioPath => {
                let Some(path) = FileDialog::new().pick_file() else {
                    return;
                };

                let Ok(file) = File::open(path) else { return };

                let source = Decoder::try_from(BufReader::new(file)).unwrap().buffered();

                self.source = Some(Box::new(source.clone()));
                self.sink.append(source.clone());
            }
            Play => self.sink.play(),
            Pause => self.sink.pause(),
            PositionChanged(x) => match &self.source {
                Some(source) => {
                    let total = source.total_duration().unwrap().as_secs_f32();
                    let duration = Duration::from_secs_f32(total * x);
                    let _ = self.sink.try_seek(duration);
                }
                None => {}
            },
            VolumeChanged(volume) => {
                self.sink.set_volume(volume as f32 / 100.0);
                self.ui_volume = volume;
            }
            Tick(_) => {}
        }
    }

    fn subscription(&self) -> Subscription<KantaMessage> {
        frames().map(KantaMessage::Tick)
    }
}
