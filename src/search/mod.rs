use anyhow::Result;
use meilisearch_sdk::Client as MeiliClient;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Game {
  pub id: usize,
  pub name: String,
  pub paths: Vec<String>,
  size: String,
}

// only need to be run once
#[tracing::instrument(skip_all)]
pub async fn init_index(client: &MeiliClient) -> Result<()> {
  #[derive(Deserialize)]
  pub struct GameResp {
    pub name: String,
    pub size: String,
  }

  const SEARCH_ENDPOINT: &str = "https://www.shinnku.com/api/search";

  tracing::info!("Fetching games...");

  let games: Vec<GameResp> = Client::new()
    .get(SEARCH_ENDPOINT)
    .query(&[("q", "files")]) // 😇
    .send()
    .await?
    .json()
    .await?;

  tracing::info!(num = games.len(), "Games fetched");

  let games: Vec<_> = games
    .into_iter()
    .enumerate()
    .filter_map(|(id, game)| {
      let mut paths: Vec<_> = game.name.split('/').map(|p| p.to_owned()).collect();
      if paths[0] == "" {
        paths.remove(0);
      } else {
        tracing::warn!("No prefix empty string");
      }

      if paths.is_empty() {
        tracing::error!(game = game.name, "Can not split into paths");
        return None;
      }

      let name = paths.last().unwrap().clone();

      Some(Game {
        id,
        name,
        paths,
        size: game.size,
      })
    })
    .collect();

  tracing::info!(num = games.len(), "Adding documents");

  client.index("games").add_documents(&games, None).await?;

  Ok(())
}
