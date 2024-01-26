use atri_core::{init_index, setup_tracing, tracing, MeiliClient};

#[tokio::main]
async fn main() {
  setup_tracing();

  let client = MeiliClient::new("http://localhost:7700", None::<&str>);

  if let Err(error) = init_index(&client).await {
    tracing::error!(%error, "Failed!");
  }
}
