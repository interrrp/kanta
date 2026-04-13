#![deny(clippy::all)]

use std::time::Duration;

use iced::{
    alignment::Vertical,
    time,
    widget::{button, column, row, scrollable, slider, text},
    Color, Element, Length, Padding, Pixels, Settings, Subscription,
};
use rfd::FileDialog;

mod media_controls;
mod player;
mod track;

use player::Player;
use track::Track;

const MUTED_COLOR: Color = Color::from_rgb(0.75, 0.75, 0.75);
const SELECTED_COLOR: Color = Color::from_rgb(0.5, 1.0, 0.5);

struct Kanta {
    player: Player,
    error: Option<String>,
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
            player: Player::try_new().unwrap_or_else(|e| {
                eprintln!("Failed to initialize audio: {}", e);
                Player::default()
            }),
            error: None,
        }
    }

    fn update(&mut self, message: KantaMessage) {
        use KantaMessage::*;

        let result = match message {
            Play => self.player.play(),
            Pause => self.player.pause(),
            JumpToPreviousTrack => self.player.jump_to_previous_track(),
            JumpToNextTrack => self.player.jump_to_next_track(),
            JumpToTrack(index) => self.player.jump_to_track_at(index),
            ClearPlaylist => self.player.clear_playlist(),
            SetPosition(position) => self.player.set_position(Duration::from_secs_f32(position)),
            SetVolume(volume) => {
                self.player.set_volume(volume);
                Ok(())
            }
            Tick => self.player.tick(),

            AddTrack => {
                if let Some(path) = FileDialog::new()
                    .set_title("Add track")
                    .add_filter("Tracks", &["mp3", "ogg", "wav", "flac"])
                    .pick_file()
                {
                    match Track::load(path).map_err(|e| e.to_string()) {
                        Ok(track) => self.player.add_to_playlist(track),
                        Err(e) => {
                            self.error = Some(e);
                        }
                    }
                }
                return;
            }

            LoadPlaylist => {
                if let Some(path) = FileDialog::new()
                    .set_title("Load playlist")
                    .add_filter("Playlists", &["m3u8"])
                    .pick_file()
                {
                    if let Err(e) = self.player.load_m3u8_playlist(path.as_path()) {
                        self.error = Some(e.to_string());
                    }
                }
                return;
            }

            ExportPlaylist => {
                if let Some(path) = FileDialog::new()
                    .set_title("Export playlist")
                    .add_filter("Playlists", &["m3u8"])
                    .save_file()
                {
                    if let Err(e) = self.player.export_m3u8_playlist(path.as_path()) {
                        self.error = Some(e.to_string());
                    }
                }
                return;
            }
        };

        if let Err(e) = result {
            self.error = Some(e.to_string());
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

        let muted = MUTED_COLOR;
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
                SELECTED_COLOR
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

            let path_str = track
                .path()
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");

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

    fn subscription(&self) -> Subscription<KantaMessage> {
        time::every(Duration::from_millis(100)).map(|_| KantaMessage::Tick)
    }
}

fn main() -> iced::Result {
    iced::application(Kanta::new, Kanta::update, Kanta::view)
        .subscription(Kanta::subscription)
        .title("Kanta")
        .settings(Settings {
            default_text_size: Pixels(14.0),
            ..Default::default()
        })
        .run()
}
