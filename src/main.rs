#![deny(clippy::all)]

use std::time::Duration;

use iced::{
    Color, Element, Length, Padding, Pixels, Settings, Subscription,
    alignment::Vertical,
    application, time,
    widget::{button, column, row, scrollable, slider, text},
};
use rfd::FileDialog;

use crate::player::Player;
use crate::track::Track;

mod player;
mod track;

fn main() -> iced::Result {
    application(Kanta::new, Kanta::update, Kanta::view)
        .subscription(Kanta::subscription)
        .title("Kanta")
        .window_size((640, 360))
        .settings(Settings {
            default_text_size: Pixels(14.0),
            ..Default::default()
        })
        .run()
}

struct Kanta {
    player: Player,
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
        Kanta {
            player: Player::new(),
        }
    }

    fn view(&self) -> Element<'_, KantaMessage> {
        let controls = {
            let play_pause_button = if self.player.current_track().is_some() {
                if self.player.is_paused() {
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

            let position_slider = match self.player.pos() {
                Some(pos) => slider(0.0..=1.0, pos, KantaMessage::PositionChanged).step(0.01),
                None => slider(0.0..=1.0, 0.0, KantaMessage::PositionChanged),
            };

            let volume_slider =
                slider(0.0..=1.0, self.player.volume(), KantaMessage::VolumeChanged).step(0.01);

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
            .player
            .current_track()
            .as_ref()
            .and_then(|track| track.lyrics())
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

        let mut queue_songs = column![].spacing(16);
        for (index, track) in self.player.queue().iter().enumerate() {
            queue_songs = queue_songs.push(
                button(
                    column![]
                        .push(text(track.title()).size(Pixels(16.0)))
                        .push(text(track.album().unwrap_or("No album")).size(Pixels(14.0)))
                        .push(text(track.artist().unwrap_or("No artist")).size(Pixels(12.0)))
                        .spacing(2)
                        .padding(Padding {
                            left: if self.player.queue_pos() == Some(index) {
                                16.0
                            } else {
                                2.0
                            },
                            top: 0.0,
                            bottom: 0.0,
                            right: 0.0,
                        }),
                )
                .on_press(KantaMessage::Jump(index))
                .style(button::text)
                .padding(0),
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
                self.player.add_to_queue(track);
            }
            Play => self.player.play(),
            Pause => self.player.pause(),
            Prev => self.player.prev(),
            Next => self.player.next(),
            Jump(pos) => self.player.jump(pos),
            ClearQueue => self.player.clear(),
            PositionChanged(pos) => self.player.set_pos(pos),
            VolumeChanged(volume) => self.player.set_volume(volume),
            Tick => {
                if self.player.is_idle() {
                    self.player.next();
                }
            }
        }
    }

    fn subscription(&self) -> Subscription<KantaMessage> {
        time::every(Duration::from_millis(10)).map(|_| KantaMessage::Tick)
    }
}
