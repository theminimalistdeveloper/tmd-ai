mod http_handler;
use http_handler::function_handler;
use lambda_http::{Error, service_fn, tracing};
use aws_sdk_bedrockruntime::Client as BedrockClient;


#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let bedrock_client = BedrockClient::new(&config);

    lambda_runtime::run(service_fn(|event| function_handler(event, &bedrock_client))).await?;
    Ok(())
}
