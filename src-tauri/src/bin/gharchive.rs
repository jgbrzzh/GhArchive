#[tokio::main]
async fn main() {
    std::process::exit(gharchive::cli::run().await);
}
