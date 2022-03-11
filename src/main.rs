use std::sync::Arc;

use dotenv::dotenv;
use futures::TryFutureExt;
use lazy_static::lazy_static;
use regex::Regex;
use rspotify::{
    clients::{BaseClient, OAuthClient},
    model::{Id, PlayableId, PlaylistId, TrackId},
    AuthCodeSpotify,
};
use teloxide::{
    adaptors::AutoSend,
    prelude::{Requester, RequesterExt},
    respond,
    types::{Chat, MediaKind, MediaText, Message, MessageCommon, MessageKind},
    Bot,
};

use crate::client::get_client;
pub mod client;

#[tokio::main]
async fn main() {
    dotenv().ok();
    teloxide::enable_logging!();
    log::info!("Starting bot...");

    let spotify = Arc::new(get_client());
    let bot = Bot::from_env().auto_send();

    teloxide::repls2::repl(bot, {
        move |msg: Message, bot: AutoSend<Bot>| {
            let spotify = spotify.clone();
            async move {
                let extracted_media_text = extract_media_text(&msg).map(|(chat_id, message)| {
                    (format!("telegram-{chat_id}"), extract_spotify_urls(message))
                });

                let (chat_id, track_ids) = match extracted_media_text {
                    Some(tuple) => tuple,
                    None => {
                        log::info!("no media text in {msg:?}");
                        return respond(());
                    }
                };

                log::info!("got media text: {track_ids:?}");

                if track_ids.len() == 0 {
                    log::info!("Found no track ids in message, skipping.");
                    return respond(());
                }

                let add_items_result = add_items_to_playlist(chat_id, spotify.as_ref(), track_ids);

                let message = match add_items_result {
                    Err(e) => e,
                    Ok(e) => e,
                };

                let send_message_result = bot
                    .send_message(msg.chat.id, message)
                    .map_err(|err| err.to_string())
                    .await;

                match send_message_result {
                    Ok(_) => log::info!("Sent message successfully!"),
                    Err(e) => log::error!("Error when sending message: {e}"),
                };
                respond(())
            }
        }
    })
    .await;
}

fn add_items_to_playlist(
    expected_name: String,
    spotify: &AuthCodeSpotify,
    track_ids: Vec<String>,
) -> Result<String, String> {
    if track_ids.len() == 0 {
        log::info!("Found no track ids in message, skipping.");
        return Err("No items to add.".to_string());
    }

    let parsing_result = track_ids
        .into_iter()
        .map(|track_id| TrackId::from_id(&track_id))
        .collect::<Result<Vec<_>, _>>();

    let sliced = match parsing_result {
        Ok(sliced) => sliced,
        Err(e) => return Err(format!("Could not parse tracks: {e}")),
    };

    let playable = sliced
        .iter()
        .map(|x| x as &dyn PlayableId)
        .collect::<Vec<_>>();

    let user_playlists = match spotify
        .current_user_playlists()
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(res) => res,
        Err(e) => return Err(format!("Unable to get my playlists: {e}").to_string()),
    };

    let current_user = match spotify.current_user() {
        Ok(user) => user,
        Err(e) => return Err(format!("Unable to get current user: {e}")),
    };

    let found_playlist_id = {
        let found_playlist = user_playlists.into_iter().find(|x| x.name == expected_name);
        if let Some(playlist_id) = found_playlist {
            playlist_id.id
        } else {
            let created_playlist = spotify
                .user_playlist_create(
                    &current_user.id,
                    &expected_name,
                    Some(true),
                    Some(false),
                    None,
                )
                .map(|x| x.id);
            match created_playlist {
                Ok(id) => id,
                Err(e) => return Err(format!("Unable to create playlist: {e}").to_string()),
            }
        }
    };

    let playlist_options = match spotify
        .playlist_items(&found_playlist_id, None, None)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(playlist) => playlist,
        Err(e) => return Err(format!("Could not get user playlist: {e}").to_string()),
    }
    .into_iter()
    .map(|x| x.track.and_then(|x| x.id().map(|x| x.id().to_string())))
    .collect::<Option<Vec<_>>>();

    let playlist = match playlist_options {
        Some(items) => items,
        None => return Err(format!("Could not get track ids.").to_string()),
    };

    let playables_to_add = playable
        .into_iter()
        .filter(|playable| !playlist.clone().into_iter().any(|x| x == playable.id()))
        .collect::<Vec<_>>();

    let playlist_url = match PlaylistId::from_id_or_uri(found_playlist_id.id()) {
        Ok(playlist) => playlist.url(),
        Err(e) => return Err(format!("Could not format playlist URL: {e}")),
    };

    if playables_to_add.len() < 1 {
        Err("No items to add, or only duplicates.".to_string())
    } else {
        let res = spotify
            .playlist_add_items(&found_playlist_id, playables_to_add, None)
            .map_err(|err| format!("Could not add items: {}", err.to_string()));
        res.map(|_| format!("Added to playlist: {playlist_url}").to_string())
    }
}

fn extract_media_text(message: &Message) -> Option<(i64, &str)> {
    match message {
        Message {
            chat: Chat { id, .. },
            kind:
                MessageKind::Common(MessageCommon {
                    media_kind: MediaKind::Text(MediaText { text, .. }),
                    ..
                }),
            ..
        } => Some((*id, text)),
        _ => None,
    }
}

fn extract_spotify_urls(string: &str) -> Vec<String> {
    lazy_static! {
        static ref URL_REGEX: Regex =
            Regex::new(r"https://open.spotify.com/track/([\w0-9]+)").unwrap();
    }
    URL_REGEX
        .captures_iter(&string)
        .filter_map(|cap| match cap.get(1) {
            Some(track_id) => Some(track_id.as_str().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
}

#[test]
fn test_get_spotify_url() {
    let url1 = "https://open.spotify.com/track/42i30whtcm9lGWx30x8t2R?si=a3fa0e97fbeb43f6";
    let url2 = "https://open.spotify.com/track/5Zm8huZ4tzDm7eLKFScrE8?si=4933882b136f49fc";
    let url3 = "https://open.spotify.com/track/5xzgJJGzPm2HlroRVKYOwF?si=5261dfc19ecd475f";
    let multiple_valid_urls = format!("{url1}, {url2}\n {url3}");
    assert_eq!(extract_spotify_urls(url1), vec!["42i30whtcm9lGWx30x8t2R"]);
    assert_eq!(
        extract_spotify_urls(&multiple_valid_urls),
        vec![
            "42i30whtcm9lGWx30x8t2R",
            "5Zm8huZ4tzDm7eLKFScrE8",
            "5xzgJJGzPm2HlroRVKYOwF"
        ]
    );
}
