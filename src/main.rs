use teloxide::{
    prelude2::*,
    types::{MediaKind, MediaText, MessageCommon, MessageKind},
};
extern crate dotenv;
use dotenv::dotenv;
use lazy_static::lazy_static;
use regex::Regex;

#[tokio::main]
async fn main() {
    dotenv().ok();
    teloxide::enable_logging!();
    log::info!("Starting bot...");

    let bot = Bot::from_env().auto_send();

    teloxide::repls2::repl(bot, |message: Message, bot: AutoSend<Bot>| async move {
        let track_ids = extract_media_text(&message)
            .map(extract_spotify_urls)
            .unwrap_or(vec![]);
        match bot
            .send_message(message.chat.id, format!("Found track ids: {track_ids:?}"))
            .await
        {
            Ok(_) => log::info!("Sent message successfully!"),
            Err(_) => log::info!("Failed sending message"),
        };
        respond(())
    })
    .await;
}

fn extract_media_text(message: &Message) -> Option<&str> {
    match message {
        Message {
            kind:
                MessageKind::Common(MessageCommon {
                    media_kind: MediaKind::Text(MediaText { text, .. }),
                    ..
                }),
            ..
        } => Some(text),
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
