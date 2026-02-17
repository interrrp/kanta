use std::{
    sync::mpsc::{Receiver, channel},
    time::Duration,
};

use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition};

use crate::track::Track;

pub struct KantaMediaControls {
    media_controls: MediaControls,
    event_rx: Receiver<MediaControlEvent>,
}

impl KantaMediaControls {
    pub fn try_new() -> anyhow::Result<KantaMediaControls> {
        #[cfg(not(target_os = "windows"))]
        let hwnd = None;

        #[cfg(target_os = "windows")]
        let hwnd = {
            use raw_window_handle::windows::WindowsHandle;

            let handle: WindowsHandle = unimplemented!();
            Some(handle.hwnd)
        };

        let config = souvlaki::PlatformConfig {
            dbus_name: "kanta",
            display_name: "Kanta",
            hwnd,
        };
        let mut media_controls = MediaControls::new(config)?;

        let (event_tx, event_rx) = channel();

        media_controls.attach({
            let tx = event_tx.clone();
            move |event| {
                tx.send(event).unwrap();
            }
        })?;

        Ok(KantaMediaControls {
            media_controls,
            event_rx,
        })
    }

    pub fn receive_event(&mut self) -> Option<MediaControlEvent> {
        self.event_rx.try_recv().ok()
    }

    pub fn update_metadata(&mut self, track: &Track) -> anyhow::Result<()> {
        self.media_controls.set_metadata(MediaMetadata {
            title: track.title(),
            artist: track.artist(),
            album: track.album(),
            duration: Some(track.duration()),
            ..Default::default()
        })?;
        Ok(())
    }

    pub fn update_playback(
        &mut self,
        is_idle: bool,
        is_paused: bool,
        position: Duration,
    ) -> anyhow::Result<()> {
        let progress = Some(MediaPosition(position));

        self.media_controls
            .set_playback(if !is_idle && !is_paused {
                MediaPlayback::Playing { progress }
            } else if is_idle && is_paused {
                MediaPlayback::Paused { progress }
            } else {
                MediaPlayback::Stopped
            })?;

        Ok(())
    }
}
