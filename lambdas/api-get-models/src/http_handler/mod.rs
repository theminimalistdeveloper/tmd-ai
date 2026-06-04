use lambda_http::{http, Body, Error, Response, Request, http::HeaderValue};
use aws_sdk_bedrock::Client as BedrockClient;
use serde_json::{json, Value};

pub(crate) async fn function_handler(
    event: Request,
    bedrock_client: &BedrockClient,
) -> Result<Response<Body>, Error> {
    tracing::info!("Event {:?}", event);

    let models = bedrock_client
        .list_foundation_models()
        .send()
        .await?
        .model_summaries
        .unwrap_or_default();

    let openai_models: Vec<Value> = models
        .into_iter()
        .map(|model| {
            json!({
                "id": model.model_id,
                "object": "model",
                "created": 1686935002,
                "owned_by": model.provider_name.unwrap_or_else(|| "unknown".to_string()),
                "root": model.model_id,
                "parent": Value::Null,
                "permission": json!([{
                    "id": "modelperm-xxxxxx",
                    "object": "model_permission",
                    "created": 1686935002,
                    "allow_create_engine": false,
                    "allow_sampling": true,
                    "allow_logprobs": true,
                    "allow_search_indices": false,
                    "allow_view": true,
                    "allow_fine_tuning": false,
                    "organization": "*",
                    "group": Value::Null,
                    "is_blocking": false
                }])
            })
        })
        .collect();

    let response_body = json!({
        "object": "list",
        "data": openai_models
    });

    Ok(Response::builder()
        .status(http::StatusCode::OK)
        .header(http::header::CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(Body::Text(response_body.to_string()))?)
}
