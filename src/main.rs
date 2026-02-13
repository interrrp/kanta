use std::{fs::File, io::BufReader, time::Duration};

use iced::{
    Element, Subscription, application, time,
    widget::{button, column, slider, text},
};
use rfd::FileDialog;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

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
    source: Option<Box<dyn Source>>,
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
            source: None,
        }
    }

    fn view(&self) -> Element<'_, KantaMessage> {
        let select_audio_path_button =
            button("Select audio file").on_press(KantaMessage::SelectAudioPath);

        let play_pause_button = if self.sink.is_paused() {
            button("Play").on_press(KantaMessage::Play)
        } else {
            button("Pause").on_press(KantaMessage::Pause)
        };

        let position_slider = match &self.source {
            Some(src) => {
                let elapsed = self.sink.get_pos().as_secs_f32();
                let total = src.total_duration().unwrap().as_secs_f32();

                slider(0.0..=1.0, elapsed / total, KantaMessage::PositionChanged).step(0.01)
            }
            None => slider(0.0..=100.0, 0.0, KantaMessage::PositionChanged).step(0.01),
        };

        let volume_slider =
            slider(0.0..=1.0, self.sink.volume(), KantaMessage::VolumeChanged).step(0.01);

        column![
            select_audio_path_button,
            play_pause_button,
            text("Playback"),
            position_slider,
            text("Volume"),
            volume_slider,
        ]
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

            VolumeChanged(volume) => self.sink.set_volume(volume),

            // Tells Iced to rerender UI elements, especially the position slider
            UpdatePositionSlider => {}
        }
    }

    fn subscription(&self) -> Subscription<KantaMessage> {
        time::every(Duration::from_millis(500)).map(|_| KantaMessage::UpdatePositionSlider)
    }
}
