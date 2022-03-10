//! This example is specially useful for the OAuth tests. It simply obtains an
//! access token and a refresh token with all available scopes.
//!
//! Set RSPOTIFY_CLIENT_ID, RSPOTIFY_CLIENT_SECRET and RSPOTIFY_REDIRECT_URI in
//! an .env file or export them manually as environmental variables for this to
//! work.

#[macro_use]
extern crate rouille;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::{fs::OpenOptions};
use std::io::{prelude::*, self};

use pretty_env_logger::env_logger;
use rspotify::{prelude::*, scopes, AuthCodeSpotify, Credentials, OAuth};

#[tokio::main]
async fn main() {
    // You can use any logger for debugging.
    env_logger::init();

    // The credentials must be available in the environment. Enable
    // `env-file` in order to read them from an `.env` file.
    let creds = Credentials::from_env().unwrap();

    // Using every possible scope
    let scopes = scopes!(
        "user-read-email",
        "user-read-private",
        "user-top-read",
        "user-read-recently-played",
        "user-follow-read",
        "user-library-read",
        "user-read-currently-playing",
        "user-read-playback-state",
        "user-read-playback-position",
        "playlist-read-collaborative",
        "playlist-read-private",
        "user-follow-modify",
        "user-library-modify",
        "user-modify-playback-state",
        "playlist-modify-public",
        "playlist-modify-private",
        "ugc-image-upload"
    );
    let oauth = OAuth {
        redirect_uri: "http://localhost:8888/callback".to_string(),
        scopes,
        ..Default::default()
    };

    let mut spotify = AuthCodeSpotify::new(creds, oauth);

    tokio::task::spawn(run_server(spotify.clone()));

    let url = spotify.get_authorize_url(false).unwrap();

    spotify.prompt_for_token(&url).unwrap();

    let token = spotify.token.lock().unwrap();
    println!(
        "RSPOTIFY_ACCESS_TOKEN={}",
        &token.as_ref().unwrap().access_token
    );
    println!(
        "RSPOTIFY_REFRESH_TOKEN={}",
        token.as_ref().unwrap().refresh_token.as_ref().unwrap()
    );
    
}


async fn run_server(spotify: AuthCodeSpotify) {
    rouille::start_server("localhost:8888", move |request| {
        router!(request,
            (GET) (/callback) => {
                let mut full_url: String = "http://localhost:8888".to_owned();
                let url = request.raw_url();
                full_url.push_str(&url);
                let code = &spotify.parse_response_code(&full_url).unwrap();
                let mut other_spotify = spotify.clone();
                other_spotify.request_token(code).expect("request_token error");

                let token = other_spotify.token.lock().unwrap();

                let mut env_file_lines: HashMap<String, String> = HashMap::new();

                // File hosts must exist in current path before this produces output
                if let Ok(lines) = read_lines("./.env") {
                    // Consumes the iterator, returns an (Optional) String
                    for line in lines {
                        match line {
                            Ok(input) =>  {
                                let split_str: Vec<&str> = input.split("=").collect();
                                env_file_lines.insert(split_str[0].trim().to_string(), split_str[1].trim().to_string());
                            },
                            Err(e) => {
                                eprintln!("Couldn't read line from file: {}", e);
                            }
                        };
                    }
                }

                env_file_lines.insert("RSPOTIFY_ACCESS_TOKEN".to_string(), format!("\"{}\"", token.as_ref().unwrap().access_token));
                env_file_lines.insert("RSPOTIFY_REFRESH_TOKEN".to_string(), format!("\"{}\"", token.as_ref().unwrap().refresh_token.as_ref().unwrap()));

                let mut file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(".env")
                    .unwrap();

                if let Err(e) = file.set_len(0) {
                    eprintln!("Couldn't clear file: {}", e);
                }

                for (key, value) in &env_file_lines {
                    if let Err(e) = writeln!(file,  "{}", [key.as_str(), value].join("=")) {
                        eprintln!("Couldn't write to file: {}", e);
                    }
                }

                println!("Done updating .env file. You may close the browser window.");

                std::process::exit(0);
                #[allow(unreachable_code)]
                rouille::Response::text("request received on /callback")
            },
            _ => rouille::Response::empty_404()
        )
    });
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}