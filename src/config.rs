use std::env;
use ron::{ser, de};
use ron::ser::PrettyConfig;
use serde::{Serialize, Deserialize};
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    token: String,
    prefix: String,
    author_id: u64,
    spotify_client_id: String,
    spotify_client_secret: String,
    spotify_redirect_uri: String,
}

impl Config {
    pub fn new() -> Self {
        dotenv::dotenv().expect("Failed to load .env file.");
        let dc_token = env::var("TOKEN").unwrap();
        let client_id = env::var("CLIENT_ID").unwrap();
        let client_secret = env::var("CLIENT_SECRET").unwrap();
        let redirect_uri = env::var("SPOTIFY_REDIRECT_URI").unwrap();
        let aut_id = env::var("AUTHOR_ID").unwrap();

      return Config {
          token: dc_token,
          prefix: String::from("!"),
          author_id: aut_id.parse::<u64>().unwrap(),
          spotify_client_id: client_id,
          spotify_client_secret: client_secret,
          spotify_redirect_uri: redirect_uri,
      }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let pretty = PrettyConfig::new()
            .depth_limit(2)
            .separate_tuple_members(true)
            .enumerate_arrays(true);
        let serialized = ser::to_string_pretty(&self, pretty)
            .expect("Serialization failed!");
        let mut file = std::fs::File::create("config.ron")?;

        if let Err(error) = write!(file, "{}", serialized) {
            println!("Failed writing to file: {}", error);
        } else {
            println!("Write operation succeeded!");
        }

        return Ok(())
    }

    pub fn load() -> std::io::Result<Config> {
        let input_path = format!("{}/config.ron", env!("CARGO_MANIFEST_DIR"));
        let file = std::fs::File::open(&input_path)
            .expect("Failed to open the file!");
        let config: Config = match de::from_reader(file) {
            Ok(cfg) => cfg,
            Err(error) => {
                println!("Failed to load config: {}", error);
                std::process::exit(1);
            }
        };

        return Ok(config)
    }

    pub fn token(&self) -> &String { return &self.token; }

    pub fn prefix(&self) -> &String { return &self.prefix; }

    pub fn spotify_client_id(&self) -> &String { return &self.spotify_client_id; }

    pub fn spotify_client_secret(&self) -> &String { return &self.spotify_client_secret; }

    pub fn spotify_redirect_uri(&self) -> &String { return &self.spotify_redirect_uri; }
}