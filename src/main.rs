mod search;
mod setup;

use meilisearch_sdk::Client as MeiliClient;
use teloxide::{
  adaptors::{throttle::Limits, CacheMe, Throttle},
  prelude::*,
  types::ParseMode,
  utils::html::link,
};

use search::Game;
use setup::setup_tracing;

#[tokio::main]
async fn main() {
  setup_tracing();

  type AtriBot = CacheMe<Throttle<Bot>>;

  let client = MeiliClient::new("http://localhost:7700", None::<&str>);

  let bot: AtriBot = Bot::from_env().throttle(Limits::default()).cache_me();

  let handler = Update::filter_message().endpoint(
    |bot: AtriBot, client: MeiliClient, msg: Message| async move {
      if let Some(keyword) = msg.text() {
        tracing::info!(keyword, "Start searching...");

        let Ok(res) = client
          .index("games")
          .search()
          .with_query(keyword)
          .execute::<Game>()
          .await
        else {
          bot.send_message(msg.chat.id, "搜索失败了！").await?;
          return respond(());
        };

        tracing::info!(num = res.hits.len(), "Search finished");

        if res.hits.is_empty() {
          bot.send_message(msg.chat.id, "什么都没有找到！").await?;
          return respond(());
        }

        let res = res
          .hits
          .into_iter()
          .take(10)
          .map(|game| {
            let game = game.result;

            format!(
              "{}. {}",
              game.id,
              link(
                &format!(
                  "https://www.shinnku.com/api/download/{}",
                  game.paths.join("/")
                ),
                &game.name,
              )
            )
          })
          .fold(String::new(), |acc, x| format!("{acc}{x}\n"));

        bot
          .send_message(msg.chat.id, res)
          .parse_mode(ParseMode::Html)
          .await?;
      }

      respond(())
    },
  );

  tracing::info!("Listening...");

  Dispatcher::builder(bot, handler)
    .dependencies(dptree::deps![client])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}
