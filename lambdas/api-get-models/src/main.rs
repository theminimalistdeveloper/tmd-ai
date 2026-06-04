mod http_handler;
use aws_sdk_bedrock::Client as BedrockClient;
use http_handler::function_handler;
use lambda_http::{Error, run, service_fn, tracing};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let bedrock_client = BedrockClient::new(&config);

    run(service_fn(|event| function_handler(event, &bedrock_client))).await?;
    Ok(())
}
