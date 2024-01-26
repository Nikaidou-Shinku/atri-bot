mod constants;
mod handler;
mod state;

use std::sync::{atomic::AtomicUsize, Arc};

use atri_core::{setup_tracing, tracing, MeiliClient};
use lru::LruCache;
use parking_lot::Mutex;
use teloxide::{
  adaptors::{throttle::Limits, CacheMe, Throttle},
  prelude::*,
};

use handler::{callback_handler, message_handler};
use state::AtriState;

type AtriBot = CacheMe<Throttle<Bot>>;

#[tokio::main]
async fn main() {
  setup_tracing();

  let state = Arc::new(AtriState {
    count: AtomicUsize::new(0),
    searches: Mutex::new(LruCache::new(1000.try_into().unwrap())),
    meili_client: MeiliClient::new("http://localhost:7700", None::<&str>),
  });

  let bot = Bot::from_env().throttle(Limits::default()).cache_me();

  let handler = dptree::entry()
    .branch(Update::filter_message().endpoint(message_handler))
    .branch(Update::filter_callback_query().endpoint(callback_handler));

  tracing::info!("Listening...");

  Dispatcher::builder(bot, handler)
    .dependencies(dptree::deps![state])
    .build()
    .dispatch()
    .await;
}
