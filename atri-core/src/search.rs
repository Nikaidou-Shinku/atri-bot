use anyhow::Result;
use meilisearch_sdk::Client as MeiliClient;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::SearchResults;

#[derive(Deserialize, Serialize)]
pub struct Game {
  pub id: usize,
  pub name: String,
  pub paths: Vec<String>,
  pub size: String,
}

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
    .enumerate() // TODO: maybe use hash as primary key
    .filter_map(|(id, game)| {
      let mut paths: Vec<_> = game.name.split('/').map(|p| p.to_owned()).collect();
      if paths[0] == "" {
        paths.remove(0);
      } else {
        tracing::warn!(game = game.name, "No prefix empty string");
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

pub async fn search(
  client: &MeiliClient,
  keyword: impl AsRef<str>,
  limit: usize,
  offset: usize,
) -> Result<SearchResults> {
  Ok(
    client
      .index("games")
      .search()
      .with_query(keyword.as_ref())
      .with_limit(limit)
      .with_offset(offset)
      .execute::<Game>()
      .await?,
  )
}
