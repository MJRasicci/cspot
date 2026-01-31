/*
// TODO: This is the rust discovery playback example, we want to rewrite this to use our c-bindings.

use std::{env, process::exit};

use data_encoding::HEXLOWER;
use futures_util::StreamExt;
use librespot::{
    connect::{ConnectConfig, LoadRequest, LoadRequestOptions, Spirc},
    core::{config::SessionConfig, session::Session, spotify_id::SpotifyId, spotify_uri::SpotifyUri},
    discovery::{DeviceType, Discovery},
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        mixer::{self, MixerConfig},
        player::Player,
    },
};
use sha1::{Digest, Sha1};

fn compute_device_id(name: &str) -> String {
    HEXLOWER.encode(&Sha1::digest(name.as_bytes()))
}

fn parse_track(input: &str) -> SpotifyUri {
    if let Ok(uri) = SpotifyUri::from_uri(input) {
        if matches!(uri, SpotifyUri::Track { .. }) {
            return uri;
        }
        eprintln!(
            "TRACK must be a Spotify track URI like \"spotify:track:2WUy2Uywcj5cP0IXQagO3z\" \
             or a base62 track id."
        );
        exit(1);
    }

    let id = SpotifyId::from_base62(input).unwrap_or_else(|_| {
        eprintln!(
            "TRACK must be a Spotify URI like \"spotify:track:2WUy2Uywcj5cP0IXQagO3z\" \
             or a base62 track id."
        );
        exit(1);
    });

    SpotifyUri::Track { id }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args: Vec<_> = env::args().collect();
    if args.len() > 2 {
        eprintln!("Usage: {} [TRACK]", args[0]);
        eprintln!("TRACK can be a Spotify URI (spotify:track:...) or a base62 track id.");
        return;
    }

    let track = args.get(1).map(|arg| parse_track(arg));

    let device_name = "Librespot Discovery Playback";
    let device_id = compute_device_id(device_name);

    let mut session_config = SessionConfig::default();
    session_config.device_id = device_id.clone();

    let mut discovery = Discovery::builder(device_id, session_config.client_id.clone())
        .name(device_name)
        .device_type(DeviceType::Speaker)
        .launch()
        .unwrap_or_else(|e| {
            eprintln!("Failed to start discovery: {e}");
            exit(1);
        });

    println!("Waiting for Spotify Connect credentials...");
    println!(
        "Open Spotify and choose \"{device_name}\" in the Connect list to authorize it."
    );

    let credentials = match discovery.next().await {
        Some(credentials) => credentials,
        None => {
            eprintln!("Discovery stopped before credentials were received.");
            exit(1);
        }
    };

    let session = Session::new(session_config, None);
    let session_handle = session.clone();

    let player_config = PlayerConfig::default();
    let audio_format = AudioFormat::default();
    let backend = audio_backend::find(None).unwrap();
    let mixer_builder = mixer::find(None).unwrap();
    let mixer = mixer_builder(MixerConfig::default()).unwrap_or_else(|e| {
        eprintln!("Failed to initialize mixer: {e}");
        exit(1);
    });

    let player = Player::new(player_config, session.clone(), mixer.get_soft_volume(), move || {
        backend(None, audio_format)
    });

    let mut connect_config = ConnectConfig::default();
    connect_config.name = device_name.to_string();
    connect_config.device_type = DeviceType::Speaker;

    println!("Starting Spotify Connect...");
    let (spirc, spirc_task) = Spirc::new(connect_config, session, credentials, player, mixer)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to start Connect: {e}");
            exit(1);
        });
    println!("Connected as {}.", session_handle.username());

    println!("Spotify Connect ready.");

    if let Some(track) = track {
        let options = LoadRequestOptions {
            start_playing: true,
            ..Default::default()
        };

        spirc.activate().unwrap_or_else(|e| {
            eprintln!("Failed to activate Connect: {e}");
            exit(1);
        });
        spirc
            .load(LoadRequest::from_tracks(vec![track.to_uri()], options))
            .unwrap_or_else(|e| {
                eprintln!("Failed to load track: {e}");
                exit(1);
            });
        spirc.play().unwrap_or_else(|e| {
            eprintln!("Failed to start playback: {e}");
            exit(1);
        });
    }

    spirc_task.await;
}
*/

#include "cspot.h"

int main(void)
{
    return 0;
}
