mod search;
mod setup;

pub use meilisearch_sdk::Client as MeiliClient;
pub use tracing;

pub use search::*;
pub use setup::*;

pub type SearchResults = meilisearch_sdk::SearchResults<Game>;
