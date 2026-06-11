use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::types::AttributeValue;
use lambda_http::Error;
use aws_lambda_events::event::apigw::{
    ApiGatewayCustomAuthorizerRequest,
    ApiGatewayCustomAuthorizerResponse,
    ApiGatewayCustomAuthorizerPolicy, 
};
use aws_lambda_events::event::iam::{IamPolicyStatement, IamPolicyEffect};
use std::collections::HashMap;

fn generate_auth_response(
    principal_id: String,
    effect: IamPolicyEffect,
    resource: String
    ) -> ApiGatewayCustomAuthorizerResponse {

    let mut response = ApiGatewayCustomAuthorizerResponse::default();
    
    response.principal_id = Some(principal_id);
    response.policy_document = ApiGatewayCustomAuthorizerPolicy::builder()
        .version("2012-10-17".to_string())
        .statement(vec![
            IamPolicyStatement::builder()
            .action(vec!["execute-api:Invoke".to_string()])
            .effect(effect)
            .resource(vec![resource])
            .build()
        ])
        .build();

    response
}

pub(crate) async fn function_handler(
    event: ApiGatewayCustomAuthorizerRequest,
    ddb: &Client
) -> Result<ApiGatewayCustomAuthorizerResponse, Error> {
    tracing::info!("Event {:?}", event);

    let table_name = std::env::var("TABLE_NAME").expect("TABLE_NAME not set");
    let method_arn = event.method_arn.unwrap_or_default();
    let authorization_token = match event.authorization_token {
        Some(a) => {
            a.trim_start_matches("Bearer ").to_string()
        },
        None => {
            return Ok(generate_auth_response(
                    "unknown".to_string(),
                    IamPolicyEffect::Deny,
                    method_arn.to_string()))
        }
    };

    let mut token_key = HashMap::new();
    token_key.insert("token_hash".to_string(), AttributeValue::S(authorization_token));

    let result = ddb
        .get_item()
        .table_name(table_name)
        .set_key(Some(token_key))
        .send()
        .await?;

    if let Some(item) = result.item {
        tracing::info!("Found item: {:?}", item);
        if let Some(AttributeValue::S(username)) = item.get("username") {
            tracing::info!("Username {}", username.clone().to_string());
            return Ok(generate_auth_response(
                    username.to_string(),
                    IamPolicyEffect::Allow,
                    method_arn.to_string()
            ));
        } else {
            tracing::info!("Could not find the username");
            return Ok(generate_auth_response(
                    "unknown".to_string(),
                    IamPolicyEffect::Deny,
                    method_arn.to_string()
            ));
        };
    }

    tracing::info!("Default deny");
    Ok(generate_auth_response(
            "unknown".to_string(),
            IamPolicyEffect::Deny,
            "resource".to_string()
    ))
}
