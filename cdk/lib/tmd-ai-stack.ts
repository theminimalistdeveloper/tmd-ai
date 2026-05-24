import * as cdk from 'aws-cdk-lib';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as ssm from 'aws-cdk-lib/aws-ssm';
import type { Construct } from 'constructs';
import { RustFunction } from 'cargo-lambda-cdk';

export class TmdAIStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const secureSecret = ssm.StringParameter.fromStringParameterName(this, 'ProxySecret', '/tmd-ai/ai-proxy/secret-token');

    const folder = `${__dirname}/../../lambdas/`;
    const proxyFunction = new RustFunction(this, 'ApiBedrockProxyFunction', {
      manifestPath:  `${folder}api-bedrock-proxy`, 
      runtime: 'provided.al2023',
      architecture: lambda.Architecture.ARM_64,
      reservedConcurrentExecutions: 2,
      timeout: cdk.Duration.minutes(3),
      memorySize: 256,
      environment: {
        RUST_LOG: 'info',
        CUSTOM_SECRET: secureSecret.stringValue
      },
    });

    proxyFunction.addToRolePolicy(
      new iam.PolicyStatement({
        actions: [
          'bedrock:InvokeModel',
          'bedrock:InvokeModelWithResponseStream',
          'bedrock:Converse',
          'bedrock:ConverseStream',
        ],
        resources: ['*'], // Bedrock operates model permissions via global ARNs by default
      })
    );

    const functionUrl = proxyFunction.addFunctionUrl({
      authType: lambda.FunctionUrlAuthType.NONE,
      cors: {
        allowedOrigins: ['*'],
        allowedHeaders: ['authorization', 'content-type'],
        allowedMethods: [lambda.HttpMethod.POST],
      },
      invokeMode: lambda.InvokeMode.RESPONSE_STREAM, 
    });

    // exports
    new cdk.CfnOutput(this, 'BedrockProxyUrl', {
      value: functionUrl.url,
      description: 'The streaming endpoint URL for CodeCompanion',
    });
  }
}
