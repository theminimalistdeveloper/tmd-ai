mod http_handler;
use aws_sdk_dynamodb::Client;
use http_handler::function_handler;
use lambda_http::tracing;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use aws_lambda_events::apigw::ApiGatewayCustomAuthorizerRequest;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = Client::new(&config);

    lambda_runtime::run(service_fn(|event: LambdaEvent<ApiGatewayCustomAuthorizerRequest>| 
            function_handler(event.payload, &client))).await?;
    Ok(())
}
