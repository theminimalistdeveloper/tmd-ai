import * as cdk from 'aws-cdk-lib';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as apigw from 'aws-cdk-lib/aws-apigateway';
import * as ddb from 'aws-cdk-lib/aws-dynamodb';
import type { Construct } from 'constructs';
import { RustFunction } from 'cargo-lambda-cdk';

export class TmdAIStack extends cdk.Stack {
  private readonly folder: string;
  private api: apigw.RestApi;
  private rootResource: apigw.IResource;
  private authorizer: apigw.IAuthorizer;
  private dynamoDBTable: ddb.ITable;
  
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    this.folder = `${__dirname}/../../lambdas/`;
    this.api = new apigw.RestApi(this, 'tmd-api-api', {
        restApiName: 'TMD AI API',
        defaultCorsPreflightOptions: {
          allowOrigins: ['*'],
          allowHeaders: ['authorization', 'content-type'],
          allowMethods: ['POST'],
        }
    });
    this.rootResource = this.api.root.addResource('v1');

    this.setDynamoDBTable();
    this.setAuthorizer();

    this.setChatCompletionEndpoint();
    this.setModelsEndpoint();

    new cdk.CfnOutput(this, 'TMDAIAPIEndpoint', {
      value: this.api.url,
      description: 'TMD AI API Gateway endpoint',
    });
  }

  setModelsEndpoint() {
    this.rootResource
    .addResource('models')
    .addMethod('GET', new apigw.LambdaIntegration(this.setGetModelsFunction()), {
      authorizer:  this.authorizer,
      authorizationType: apigw.AuthorizationType.CUSTOM,
    })
  }

  setGetModelsFunction(): lambda.IFunction {
    const getModelsFunction = new RustFunction(this, 'ApiGetModelsFunction', {
      manifestPath:  `${this.folder}api-get-models`, 
      runtime: 'provided.al2023',
      architecture: lambda.Architecture.ARM_64,
      reservedConcurrentExecutions: 2,
      timeout: cdk.Duration.seconds(10),
      memorySize: 256,
      environment: {
        RUST_LOG: 'info',
      },
    })
    
    getModelsFunction.addToRolePolicy(
      new iam.PolicyStatement({
        actions: [ 'bedrock:ListFoundationModels'],
        resources: ['*']
      }
    ));

    return getModelsFunction;
  }

  setDynamoDBTable() {
    this.dynamoDBTable = new ddb.Table(this, 'ApiKeys', {
      partitionKey: { name: 'token_hash', type: ddb.AttributeType.STRING },
      billingMode: ddb.BillingMode.PAY_PER_REQUEST,
      encryption: ddb.TableEncryption.AWS_MANAGED,
    });
  }

  setChatCompletionEndpoint() {
    this.rootResource
    .addResource('chat')
    .addResource('completions')
    .addMethod('POST', new apigw.LambdaIntegration(this.setChatCompletionsFunction(), {
      responseTransferMode: apigw.ResponseTransferMode.STREAM
    }), {
      authorizer: this.authorizer,
      authorizationType: apigw.AuthorizationType.CUSTOM,
    });
  }

  setChatCompletionsFunction(): lambda.IFunction {
    const proxyFunction = new RustFunction(this, 'ApiChatCompletionsFunction', {
      manifestPath:  `${this.folder}api-chat-completions`, 
      runtime: 'provided.al2023',
      architecture: lambda.Architecture.ARM_64,
      reservedConcurrentExecutions: 2,
      timeout: cdk.Duration.minutes(3),
      memorySize: 256,
      environment: {
        RUST_LOG: 'info',
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
        resources: [ 
          'arn:aws:bedrock:*::foundation-model/*',
          'arn:aws:bedrock:*:627860589766:inference-profile/*',
        ],
      })
    );

    return proxyFunction;
  }
  
  setAuthorizer() {
    const authFunction = new RustFunction(this, 'AuthFunction', {
      manifestPath:  `${this.folder}api-auth`, 
      runtime: 'provided.al2023',
      architecture: lambda.Architecture.ARM_64,
      reservedConcurrentExecutions: 2,
      timeout: cdk.Duration.seconds(10),
      memorySize: 256,
      environment: {
        RUST_LOG: 'info',
        TABLE_NAME: this.dynamoDBTable.tableName
      },
    });

    this.dynamoDBTable.grantReadData(authFunction);

    this.authorizer = new apigw.TokenAuthorizer(this, 'OpenAIAuthorizer', {
      handler: authFunction,
      identitySource: apigw.IdentitySource.header('Authorization'),
      resultsCacheTtl: cdk.Duration.seconds(0),
      validationRegex: '^Bearer [a-zA-Z0-9].+$',
    });
  }
}
