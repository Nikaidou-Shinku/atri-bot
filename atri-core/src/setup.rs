use tracing::Level;
use tracing_subscriber::FmtSubscriber;

pub fn setup_tracing() {
  let subscriber = FmtSubscriber::builder()
    .with_max_level(Level::INFO)
    .finish();

  tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
