#![deny(clippy::all)]

use std::{fmt::Debug, time::Duration};

use iced::{
    Color, Element, Length, Padding, Pixels, Settings, Subscription,
    alignment::Vertical,
    application, time,
    widget::{button, column, row, scrollable, slider, text},
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
        .window_size((900, 900))
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
    JumpToPreviousTrack,
    JumpToNextTrack,
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
        use KantaMessage::*;

        macro_rules! btn {
            ($text:expr, $message:expr) => {
                button($text).on_press($message)
            };
            ($text:expr, $message:expr, $style:ident) => {
                button($text).on_press($message).style(button::$style)
            };
        }

        let play_pause_button = if self.player.current_track().is_some() {
            if self.player.is_paused() {
                btn!("Play", Play)
            } else {
                btn!("Pause", Pause)
            }
        } else {
            button("Stopped")
        };

        let position_slider = match self.player.current_track() {
            Some(track) => {
                let elapsed = self.player.position().as_secs_f32();
                let total = track.duration().as_secs_f32();
                slider(0.0..=total, elapsed, SetPosition)
            }
            None => slider(0.0..=1.0, 0.0, SetPosition),
        };

        let controls = row![]
            .push(btn!("Prev", JumpToPreviousTrack, secondary))
            .push(play_pause_button)
            .push(btn!("Next", JumpToNextTrack, secondary))
            .push(text("Position"))
            .push(position_slider)
            .push(text("Volume"))
            .push(slider(0.0..=1.0, self.player.volume(), SetVolume).step(0.01))
            .spacing(8)
            .align_y(Vertical::Center);

        let playlist_row_padding = Padding {
            top: 8.0,
            bottom: 8.0,
            left: 0.0,
            right: 0.0,
        };

        let playlist_controls = row![]
            .push(btn!("Add track", AddTrack, secondary))
            .push(btn!("Load playlist", LoadPlaylist, secondary))
            .push(btn!("Export playlist", ExportPlaylist, secondary))
            .push(btn!("Clear playlist", ClearPlaylist, danger))
            .spacing(8);

        let muted = Color::from_rgb(0.75, 0.75, 0.75);
        let header_field = |name| text(name).width(Length::Fill).color(muted);
        let playlist_header = row![]
            .push(header_field("Artist"))
            .push(header_field("Album"))
            .push(header_field("Title"))
            .push(header_field("Duration"))
            .padding(playlist_row_padding);

        let mut playlist_tracks = column![];
        for (index, track) in self.player.playlist().iter().enumerate() {
            let color = if self.player.playlist_index() == Some(index) {
                Color::from_rgb(0.5, 1.0, 0.5)
            } else {
                Color::WHITE
            };

            macro_rules! track_field {
                ($method:ident, $default:expr) => {
                    text(track.$method().unwrap_or($default))
                        .width(Length::Fill)
                        .color(color)
                };
                ($content:expr) => {
                    text($content).width(Length::Fill).color(color)
                };
            }

            let path_str = track.path().file_name().unwrap().to_str().unwrap();

            let total_seconds = track.duration().as_secs();
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;
            let duration = if hours != 0 {
                format!("{}:{:02}:{:02}", hours, minutes, seconds)
            } else {
                format!("{:02}:{:02}", minutes, seconds)
            };

            playlist_tracks = playlist_tracks.push(
                btn!(
                    row![]
                        .push(track_field!(artist, "No artist"))
                        .push(track_field!(album, "No album"))
                        .push(track_field!(title, path_str))
                        .push(track_field!(duration))
                        .padding(playlist_row_padding),
                    JumpToTrack(index),
                    text
                )
                .padding(0),
            );
        }
        let playlist_tracks = scrollable(playlist_tracks);

        let playlist = column![]
            .push(playlist_controls)
            .push(playlist_header)
            .push(playlist_tracks)
            .height(Length::Fill);

        let lyrics = scrollable(
            match self.player.current_track().and_then(|track| track.lyrics()) {
                Some(lyrics) => text(lyrics),
                None => text("No lyrics available").center().color(muted),
            }
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .height(Length::Fill);

        column![]
            .push(controls)
            .push(playlist)
            .push(lyrics)
            .spacing(8)
            .padding(8)
            .into()
    }

    fn update(&mut self, message: KantaMessage) {
        use KantaMessage::*;

        match message {
            Play => self.player.play().unwrap(),
            Pause => self.player.pause().unwrap(),
            JumpToPreviousTrack => self.player.jump_to_previous_track().unwrap(),
            JumpToNextTrack => self.player.jump_to_next_track().unwrap(),
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
