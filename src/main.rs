#![deny(clippy::all)]

use std::time::Duration;

use iced::{
    Color, Element, Length, Padding, Pixels, Settings, Subscription,
    alignment::Vertical,
    application, time,
    widget::{button, column, container, row, scrollable, slider, text},
};
use rfd::FileDialog;

use crate::player::Player;
use crate::track::Track;

mod media_controls;
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
    AddTrack,
    LoadPlaylist,
    ExportPlaylist,
    ClearPlaylist,
    Play,
    Pause,
    PreviousTrack,
    NextTrack,
    JumpToTrack(usize),
    SetPosition(f32),
    SetVolume(f32),
    Tick,
}

impl Kanta {
    fn new() -> Kanta {
        Kanta {
            player: Player::try_new().unwrap(),
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
                .on_press(KantaMessage::PreviousTrack)
                .style(button::secondary);

            let next_button = button("Next")
                .on_press(KantaMessage::NextTrack)
                .style(button::secondary);

            let position_slider = match self.player.current_track() {
                Some(track) => {
                    let elapsed = self.player.position().as_secs_f32();
                    let total = track.duration().as_secs_f32();

                    slider(0.0..=total, elapsed, KantaMessage::SetPosition)
                }
                None => slider(0.0..=1.0, 0.0, KantaMessage::SetPosition),
            };

            let volume_slider =
                slider(0.0..=1.0, self.player.volume(), KantaMessage::SetVolume).step(0.01);

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
            Some(lyrics) => scrollable(container(text(lyrics)).padding(Padding {
                top: 0.0,
                right: 18.0, // Prevent scrollbar from covering lyrics (scrollbar 10px + padding 8px)
                bottom: 0.0,
                left: 0.0,
            }))
            .width(Length::Fill),
            None => scrollable(text("No lyrics available").color(muted)).width(Length::Fill),
        };

        let playlist_controls = {
            let add_track_button = button("Add").on_press(KantaMessage::AddTrack);

            let load_playlist_button = button("Load")
                .on_press(KantaMessage::LoadPlaylist)
                .style(button::secondary);

            let export_button = button("Export")
                .on_press(KantaMessage::ExportPlaylist)
                .style(button::secondary);

            let clear_button = button("Clear")
                .on_press(KantaMessage::ClearPlaylist)
                .style(button::danger);

            row![]
                .push(add_track_button)
                .push(load_playlist_button)
                .push(export_button)
                .push(clear_button)
                .spacing(8)
        };

        let mut playlist_tracks = column![].spacing(16);
        for (index, track) in self.player.playlist().iter().enumerate() {
            let path_str = track.path().file_name().unwrap().to_str().unwrap();

            let contents = column![]
                .push(text(track.title().unwrap_or(path_str)).size(Pixels(16.0)))
                .push(text(track.album().unwrap_or("No album")).size(Pixels(14.0)))
                .push(text(track.artist().unwrap_or("No artist")).size(Pixels(12.0)))
                .spacing(2)
                .padding(Padding {
                    left: if self.player.playlist_index() == Some(index) {
                        16.0
                    } else {
                        2.0
                    },
                    top: 0.0,
                    bottom: 0.0,
                    right: 0.0,
                });

            playlist_tracks = playlist_tracks.push(
                button(contents)
                    .on_press(KantaMessage::JumpToTrack(index))
                    .style(button::text)
                    .padding(0),
            );
        }

        let playlist = column![]
            .push(playlist_controls)
            .push(scrollable(playlist_tracks).width(Length::Fill))
            .width(Length::Fill)
            .spacing(8);

        let bottom_row = row![]
            .push(lyrics)
            .push(playlist)
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
            Play => self.player.play().unwrap(),
            Pause => self.player.pause().unwrap(),
            PreviousTrack => self.player.jump_to_previous_track().unwrap(),
            NextTrack => self.player.jump_to_next_track().unwrap(),
            JumpToTrack(index) => self.player.jump_to_track_at(index).unwrap(),
            ClearPlaylist => self.player.clear_playlist().unwrap(),
            SetPosition(position) => self
                .player
                .set_position(Duration::from_secs_f32(position))
                .unwrap(),
            SetVolume(volume) => self.player.set_volume(volume),
            Tick => self.player.tick().unwrap(),

            AddTrack => {
                if let Some(path) = FileDialog::new()
                    .set_title("Add track")
                    .add_filter("Tracks", &["mp3", "ogg", "wav", "flac"])
                    .pick_file()
                {
                    self.player.add_to_playlist(Track::load(path).unwrap())
                }
            }

            LoadPlaylist => {
                if let Some(path) = FileDialog::new()
                    .set_title("Load playlist")
                    .add_filter("Playlists", &["m3u8"])
                    .pick_file()
                {
                    self.player.load_m3u8_playlist(path.as_path()).unwrap();
                }
            }

            ExportPlaylist => {
                if let Some(path) = FileDialog::new()
                    .set_title("Export playlist")
                    .add_filter("Playlists", &["m3u8"])
                    .save_file()
                {
                    self.player.export_m3u8_playlist(path.as_path()).unwrap();
                }
            }
        }
    }

    fn subscription(&self) -> Subscription<KantaMessage> {
        time::every(Duration::from_millis(10)).map(|_| KantaMessage::Tick)
    }
}
