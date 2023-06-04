use std::env;
use ron::{ser, de};
use ron::ser::PrettyConfig;
use serde::{Serialize, Deserialize};
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    token: String,
    prefix: String,
    // client_id: String,
    // client_st: String,
}

impl Config {
    pub fn new() -> Self {
        let tkn = env::var("DISCORD_TOKEN").unwrap();
      return Config {
          token: tkn,
          prefix: String::from("!"),
          // client_id: env::var("CLIENT_ID").unwrap(),
          // client_st: env::var("CLIENT_ST").unwrap(),
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

    // pub fn client_id(&self) -> &String { return &self.client_id; }
    //
    // pub fn client_st(&self) -> &String { return &self.client_st; }
}