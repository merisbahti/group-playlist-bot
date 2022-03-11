use std::sync::Arc;

use dotenv::dotenv;
use futures::{lock::Mutex, TryFutureExt};
use lazy_static::lazy_static;
use regex::Regex;
use rspotify::{
    clients::OAuthClient,
    model::{Id, PlayableId, TrackId},
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

    let spotify = Arc::new(Mutex::new(get_client()));
    let bot = Bot::from_env().auto_send();

    teloxide::repls2::repl(bot, {
        move |msg: Message, bot: AutoSend<Bot>| {
            let spotify = spotify.clone();
            async move {
                log::info!("in async");
                let spotify = spotify.lock().await;
                log::info!("gotten clientasync");

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

                let parsing_result = track_ids
                    .into_iter()
                    .map(|track_id| TrackId::from_id(&track_id))
                    .collect::<Result<Vec<_>, _>>();

                let sliced = match parsing_result {
                    Ok(sliced) => sliced,
                    Err(e) => {
                        log::error!("Error when parsing tracks: {e}");
                        return respond(());
                    }
                };

                let playable = sliced
                    .iter()
                    .map(|x| x as &dyn PlayableId)
                    .collect::<Vec<_>>();

                let expected_name = chat_id;

                let user_playlists = match spotify
                    .current_user_playlists()
                    .collect::<Result<Vec<_>, _>>()
                {
                    Ok(res) => res,
                    Err(e) => {
                        log::error!("Unable to get my playlists: {e}");
                        return respond(());
                    }
                };

                let current_user = match spotify.current_user() {
                    Ok(user) => user,
                    Err(e) => {
                        log::error!("Error getting current user: {e}");
                        return respond(());
                    }
                };

                let found_playlist_id = {
                    let found_playlist =
                        user_playlists.into_iter().find(|x| x.name == expected_name);
                    if let Some(playlist_id) = found_playlist {
                        playlist_id.id
                    } else {
                        spotify
                            .user_playlist_create(
                                &current_user.id,
                                &expected_name,
                                Some(true),
                                Some(false),
                                None,
                            )
                            .unwrap()
                            .id
                    }
                };

                let _add_items = {
                    spotify
                        .playlist_add_items(&found_playlist_id, playable, None)
                        .map_err(|err| err.to_string())
                };
                let send_message = bot
                    .send_message(msg.chat.id, format!("Added to playlist!"))
                    .map_err(|err| err.to_string())
                    .await;

                match send_message {
                    Ok(_) => log::info!("Sent message successfully!"),
                    Err(e) => log::error!("{}", e),
                };
                respond(())
            }
        }
    })
    .await;
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
