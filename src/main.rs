use std::str::FromStr;

use futures::TryFutureExt;
use rspotify::{
    clients::OAuthClient,
    model::{track, Id, PlayableId, PlaylistId, TrackId},
};
use teloxide::{
    adaptors::AutoSend,
    prelude::{Requester, RequesterExt},
    respond,
    types::{Chat, MediaKind, MediaText, Message, MessageCommon, MessageKind},
    Bot,
};
extern crate dotenv;
use dotenv::dotenv;
use lazy_static::lazy_static;
use regex::Regex;

use crate::client::get_client;
pub mod client;

#[tokio::main]
async fn main() {
    dotenv().ok();
    teloxide::enable_logging!();
    log::info!("Starting bot...");

    let bot = Bot::from_env().auto_send();

    teloxide::repls2::repl(bot, |message: Message, bot: AutoSend<Bot>| async move {
        let spotify = get_client().await;

        let extracted_media_text = extract_media_text(&message).map(|(chat_id, message)| {
            (format!("telegram-{chat_id}"), extract_spotify_urls(message))
        });

        if extracted_media_text.is_none() {
            return respond(());
        }

        let (chat_id, track_ids) = extracted_media_text.unwrap();

        if track_ids.len() == 0 {
            log::info!("Found no track ids in message, skipping.");
            return respond(());
        }

        if track_ids.len() > 1 {
            bot.send_message(
                message.chat.id,
                format!(
                    "Found more than 1 track id. Because I don't know rust I'll skip the rest."
                ),
            )
            .await
            .unwrap();
        }

        let tracks = &[TrackId::from_id(track_ids.get(0).unwrap()).unwrap()];
        let playable = tracks
            .iter()
            .map(|id| id as &dyn PlayableId)
            .collect::<Vec<&dyn PlayableId>>();

        let playlist_id = "3TMQK7Eh2XlEu4ai5QMbLw";
        let playlist = PlaylistId::from_id(playlist_id).unwrap();

        let add_items = spotify
            .playlist_add_items(&playlist, playable, None)
            .map_err(|err| err.to_string());
        let send_message = add_items.and_then(|_| {
            bot.send_message(message.chat.id, format!("Added to playlist!"))
                .map_err(|err| err.to_string())
        });

        match send_message.await {
            Ok(_) => log::info!("Sent message successfully!"),
            Err(e) => log::error!("{}", e),
        };
        respond(())
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
