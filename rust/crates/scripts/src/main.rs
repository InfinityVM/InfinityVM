use scripts::signer::ResultSigner;

#[tokio::main]
async fn main() {
    ResultSigner::run().await
}
