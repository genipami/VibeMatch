use reqwest::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use tokio::io;
//use std::{env};
use std::fs;
use base64::{engine::general_purpose, Engine as _};
use once_cell::sync::OnceCell;
use crate::fs::OpenOptions;
use std::io::Write;

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorCode {
    ExpiredAuthorization,
    BadRequest,
    ExceededRateLimit,
    ParseError,
    UnexpectedResponse, 
    NetworkError,
}


#[derive(Debug, Deserialize)]
pub struct SpotifyTopTracks {
    pub href: String,
    pub limit: u32,
    pub next: Option<String>,
    pub offset: u32,
    pub previous: Option<String>,
    pub total: u32,
    pub items: Vec<Track>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyResponse {
    pub tracks: Tracks,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tracks {
    pub href: String,
    pub limit: u32,
    pub next: Option<String>,
    pub offset: u32,
    pub previous: Option<String>,
    pub total: u32,
    pub items: Vec<Track>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Track {
    pub album: Album,
    pub artists: Vec<Artist>,
    pub available_markets: Vec<String>,
    pub disc_number: u32,
    pub duration_ms: u32,
    pub explicit: bool,
    pub external_ids: ExternalIds,
    pub external_urls: ExternalUrls,
    pub href: String,
    pub id: String,
    pub is_playable: Option<bool>,
    pub name: String,
    pub popularity: u32,
    pub preview_url: Option<String>,
    pub track_number: u32,
    #[serde(rename = "type")]
    pub track_type: String,
    pub uri: String,
    pub is_local: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Album {
    pub album_type: String,
    pub total_tracks: u32,
    pub available_markets: Vec<String>,
    pub external_urls: ExternalUrls,
    pub href: String,
    pub id: String,
    pub images: Vec<Image>,
    pub name: String,
    pub release_date: String,
    pub release_date_precision: String,
    #[serde(rename = "type")]
    pub album_type_str: String,
    pub uri: String,
    pub artists: Vec<Artist>,
    pub is_playable: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Artist {
    pub external_urls: ExternalUrls,
    pub href: String,
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub artist_type: String,
    pub uri: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExternalIds {
    pub isrc: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ExternalUrls {
    pub spotify: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Image {
    pub url: String,
    pub height: u32,
    pub width: u32,
}


#[derive(Debug, Serialize, Deserialize)]
struct AudioFeatures {
    acousticness: f64,
    analysis_url: String,
    danceability: f64,
    duration_ms: u32,
    energy: f64,
    id: String,
    instrumentalness: f64,
    key: u8,
    liveness: f64,
    loudness: f64,
    mode: u8,
    speechiness: f64,
    tempo: f64,
    time_signature: u8,
    track_href: String,
    #[serde(rename = "type")]
    object_type: String,
    uri: String,
    valence: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Genre {
    id: i16,
    name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AuthToken {
    access_token: String,
    token_type: String,
    expires_in: u64,
}



static AUTH_TOKEN: OnceCell<String> = OnceCell::new();


async fn get_auth_token() -> Result<String, ErrorCode>{
    println!("Inside get_auth_token()");
    let client: Client = Client::new();
    let url: String = format!("https://accounts.spotify.com/api/token"); 
    let client_id: String = format!("d898be5f40b04f67ab20810d39d6eff8");
    let client_secret: String = format!("42e8a67641dd44b7bd89affbd611a507");
    let credentials = format!("{}:{}", client_id, client_secret);
    let encoded_credentials = general_purpose::STANDARD.encode(credentials);

    let response = client
        .post(url)
        .header("Authorization", format!("Basic {}", encoded_credentials)) 
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("grant_type=client_credentials") 
        .send()
        .await;
    match response{
        Ok(res) =>
        {
            match res.status().as_u16() {
                200 => {
                    match res.json::<AuthToken>().await {
                        Ok(parsed) => {
                            println!("Access token: {}", parsed.access_token);
                            return Ok(parsed.access_token);},
                        Err(_) => return Err(ErrorCode::ParseError),
                    };
                },
                401 => {
                    println!("ExpiredAuthorization");
                    return Err(ErrorCode::ExpiredAuthorization);
                },
                403 => {
                    println!("BadRequest");
                    return Err(ErrorCode::BadRequest);
                },
                429 => {
                    println!("ExceededRateLimit");
                    return Err(ErrorCode::ExceededRateLimit);
                },
                _ => {
                    println!("Unexpected Response: {}", res.text().await.unwrap_or_default());
                    return Err(ErrorCode::UnexpectedResponse);
                },
            };
        },
        Err(error) => {
            println!("{}", error);
            return Err(ErrorCode::UnexpectedResponse);
        }
    }
    
}

async fn init_auth_token() {
    let token = get_auth_token().await.unwrap();
    AUTH_TOKEN.set(token).unwrap();
}
async fn get_top_tracks(term:String, limit: u16, offset: u16) -> Result<Vec<Track>, ErrorCode>
{
    let url = format!("https://api.spotify.com/v1/me/top/tracks?time_range={term}&limit={limit}&offset={offset}");
    let client = reqwest::Client::new();
    let response = client
    .get(url)
    .header(AUTHORIZATION, format!("Bearer {:?}", AUTH_TOKEN.get().unwrap()))
    .header(CONTENT_TYPE, "application/json")
    .header(ACCEPT, "application/json")
    .send()
    .await
    .unwrap();
    match response.status().as_u16() {
        200 => {
            match response.json::<SpotifyTopTracks>().await {
                Ok(parsed) => return Ok(parsed.items),
                Err(_) => return Err(ErrorCode::ParseError),
            };
        },
        401 => return Err(ErrorCode::ExpiredAuthorization),
        403 => return Err(ErrorCode::BadRequest),
        429 => return Err(ErrorCode::ExceededRateLimit),
        _ => return Err(ErrorCode::UnexpectedResponse),
    };
}

async fn get_audio_features(track_id: String) -> Result<AudioFeatures, ErrorCode>
{
    let url = format!("https://api.spotify.com/v1/audio-features/{track_id}");
    let client = reqwest::Client::new();
    let auth_token = match AUTH_TOKEN.get() {
        Some(token) => token,
        None => return Err(ErrorCode::ExpiredAuthorization),
    };

    let response = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {}", auth_token))
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .send()
        .await
        .map_err(|_| ErrorCode::NetworkError)?;
    match response.status().as_u16() {
        200 => {
            match response.json::<AudioFeatures>().await {
                Ok(parsed) => {
                    println!("{}", format!("{parsed:?}"));
                    return Ok(parsed)},
                Err(_) => return Err(ErrorCode::ParseError),
            };
        },
        401 => return Err(ErrorCode::ExpiredAuthorization),
        403 => return Err(ErrorCode::BadRequest),
        429 => return Err(ErrorCode::ExceededRateLimit),
        _ => return Err(ErrorCode::UnexpectedResponse),
    };
}

fn read_genres() -> Result<Vec<Genre>, ErrorCode>
{
    println!("Inside read_genres()");
    let file_content: Result<String, std::io::Error> = fs::read_to_string("genres.json");
    match file_content{
        Ok(content) => {
            let genres:Vec<Genre> = serde_json::from_str(&content).unwrap();
            return Ok(genres);
        },
        _ => return Err(ErrorCode::ParseError),
    }
}

async fn get_tracks(genre: Genre) -> Result<Vec<Track>, ErrorCode>
{
    println!("Inside get_tracks()");
    let url: String = format!(
        "https://api.spotify.com/v1/search?q={query}&type=track&limit=50&offset=0",
        query = genre.name
    );
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {}", AUTH_TOKEN.get().unwrap()))
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .send()
        .await
        .unwrap();
    match response.status().as_u16() {
        200 => {
            match response.json::<SpotifyResponse>().await {
                Ok(parsed) => return Ok(parsed.tracks.items),
                Err(_) => return Err(ErrorCode::ParseError),
            };
        },
        401 => return Err(ErrorCode::ExpiredAuthorization),
        403 => return Err(ErrorCode::BadRequest),
        429 => return Err(ErrorCode::ExceededRateLimit),
        _ => 
        {
            println!("Raw response: {}", &response.text().await.unwrap_or_default());
            return Err(ErrorCode::UnexpectedResponse);
        },
    };
}

async fn write_tracks(genres: Vec<Genre>) -> Result<(), io::Error>
{   
    println!("Inside write_tracks()");
    let mut file = OpenOptions::new().write(true).create(true).truncate(true).open("tracks.json")?;
    let open_brace = file.write_all(b"[");
    for genre in genres.iter() {
        println!("{}", genre.name);
        let tracks: Vec<Track> = get_tracks(genre.clone()).await.unwrap();
        let core_features:Vec<(String, String)> = tracks.iter().map(|x| (x.name.clone(), x.artists[0].name.clone())).collect();
        let json: String = serde_json::to_string(&core_features).unwrap();
        let result: Result<(), std::io::Error> = file.write_all(json.as_bytes());
        match result {
            Ok(_) => {
                let newline_result = file.write_all(b",\n");
                if let Err(err) = newline_result {
                    return Err(err);
                }
            },
            Err(err) => return Err(err),
        }
    }
    let close_brace = file.write_all(b"]");
    return Ok(());
}
#[tokio::main]
async fn main()
{
    init_auth_token().await;
    // let audio_features = get_audio_features("11dFghVXANMlKmJXsNCbNl".to_string()).await.unwrap();
    // println!("{}", format!("The audio features are: {audio_features:?}"));
    let mut genres: Vec<Genre> = vec![];
    let result_of_reading_genres: Result<Vec<Genre>, ErrorCode> = read_genres();
    match result_of_reading_genres{
        Ok(res) => genres = res,
        Err(_) => println!("There was an error reading!"),
    }
    write_tracks(genres).await.unwrap();
}
