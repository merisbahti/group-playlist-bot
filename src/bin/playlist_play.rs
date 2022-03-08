use futures::TryStreamExt;

use group_playlist_bot::client::get_client;
use pretty_env_logger::env_logger;
use rspotify::clients::OAuthClient;

extern crate dotenv;
use dotenv::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();

    let spotify = get_client().await;

    let user_playlists = spotify
        .current_user_playlists()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    //spotify.playlist_add_items(playlist_id, items, position);

    println!("{:?}", user_playlists);
}

//async fn fetch_all(current_user_playlists: std::pin::Pin<Box<dyn Stream<Item = Result<rspotify::model::SimplifiedPlaylist, rspotify::ClientError>>>>) -> _ {
//    paginator.try_collect::<Vec<_>>().await.unwrap()
//    todo!()
//}
