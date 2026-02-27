# ── Lambda Layer (firebase-admin SDK) ──────────────────────────────

resource "aws_lambda_layer_version" "firebase" {
  filename            = "${path.module}/../lambda_packages/firebase_layer.zip"
  layer_name          = "driveguard-firebase-admin"
  compatible_runtimes = ["python3.12"]
  description         = "firebase-admin + google-cloud-firestore for license validation"
  source_code_hash    = filebase64sha256("${path.module}/../lambda_packages/firebase_layer.zip")
}

# ── Credential Proxy Lambda ───────────────────────────────────────

resource "aws_lambda_function" "credential_proxy" {
  function_name    = "DriveGuardCredentialProxy"
  filename         = "${path.module}/../lambda_packages/credential_proxy.zip"
  source_code_hash = filebase64sha256("${path.module}/../lambda_packages/credential_proxy.zip")
  handler          = "lambda_credential_proxy.lambda_handler"
  runtime          = "python3.12"
  timeout          = 15
  memory_size      = var.lambda_memory
  role             = aws_iam_role.lambda_exec.arn
  layers           = [aws_lambda_layer_version.firebase.arn]

  environment {
    variables = {
      S3_BUCKET_NAME           = var.bucket_name
      S3_UPLOAD_ROLE_ARN       = aws_iam_role.s3_upload.arn
      CREDENTIAL_DURATION      = tostring(var.credential_duration)
      FIREBASE_CREDENTIALS_B64 = base64encode(file(var.firebase_credentials_file))
      FIREBASE_PROJECT_ID      = var.firebase_project_id
    }
  }
}

# ── License Manager Lambda ────────────────────────────────────────

resource "aws_lambda_function" "license_manager" {
  function_name    = "DriveGuardLicenseManager"
  filename         = "${path.module}/../lambda_packages/license_manager.zip"
  source_code_hash = filebase64sha256("${path.module}/../lambda_packages/license_manager.zip")
  handler          = "lambda_license_manager.lambda_handler"
  runtime          = "python3.12"
  timeout          = 15
  memory_size      = var.lambda_memory
  role             = aws_iam_role.lambda_exec.arn
  layers           = [aws_lambda_layer_version.firebase.arn]

  environment {
    variables = {
      FIREBASE_CREDENTIALS_B64 = base64encode(file(var.firebase_credentials_file))
      FIREBASE_PROJECT_ID      = var.firebase_project_id
    }
  }
}

# ── API Gateway (REST) ────────────────────────────────────────────

resource "aws_api_gateway_rest_api" "api" {
  name        = "DriveGuardAPI"
  description = "DriveGuard Pro credential proxy and license management API"
}

# --- /credentials ---

resource "aws_api_gateway_resource" "credentials" {
  rest_api_id = aws_api_gateway_rest_api.api.id
  parent_id   = aws_api_gateway_rest_api.api.root_resource_id
  path_part   = "credentials"
}

resource "aws_api_gateway_method" "credentials_post" {
  rest_api_id   = aws_api_gateway_rest_api.api.id
  resource_id   = aws_api_gateway_resource.credentials.id
  http_method   = "POST"
  authorization = "NONE"
}

resource "aws_api_gateway_integration" "credentials_lambda" {
  rest_api_id             = aws_api_gateway_rest_api.api.id
  resource_id             = aws_api_gateway_resource.credentials.id
  http_method             = aws_api_gateway_method.credentials_post.http_method
  integration_http_method = "POST"
  type                    = "AWS_PROXY"
  uri                     = aws_lambda_function.credential_proxy.invoke_arn
}

# CORS OPTIONS for /credentials
resource "aws_api_gateway_method" "credentials_options" {
  rest_api_id   = aws_api_gateway_rest_api.api.id
  resource_id   = aws_api_gateway_resource.credentials.id
  http_method   = "OPTIONS"
  authorization = "NONE"
}

resource "aws_api_gateway_integration" "credentials_options" {
  rest_api_id = aws_api_gateway_rest_api.api.id
  resource_id = aws_api_gateway_resource.credentials.id
  http_method = aws_api_gateway_method.credentials_options.http_method
  type        = "MOCK"
  request_templates = {
    "application/json" = "{\"statusCode\": 200}"
  }
}

resource "aws_api_gateway_method_response" "credentials_options_200" {
  rest_api_id = aws_api_gateway_rest_api.api.id
  resource_id = aws_api_gateway_resource.credentials.id
  http_method = aws_api_gateway_method.credentials_options.http_method
  status_code = "200"
  response_parameters = {
    "method.response.header.Access-Control-Allow-Headers" = true
    "method.response.header.Access-Control-Allow-Methods" = true
    "method.response.header.Access-Control-Allow-Origin"  = true
  }
}

resource "aws_api_gateway_integration_response" "credentials_options_200" {
  rest_api_id = aws_api_gateway_rest_api.api.id
  resource_id = aws_api_gateway_resource.credentials.id
  http_method = aws_api_gateway_method.credentials_options.http_method
  status_code = aws_api_gateway_method_response.credentials_options_200.status_code
  response_parameters = {
    "method.response.header.Access-Control-Allow-Headers" = "'Content-Type'"
    "method.response.header.Access-Control-Allow-Methods" = "'POST,OPTIONS'"
    "method.response.header.Access-Control-Allow-Origin"  = "'*'"
  }
}

# --- /licenses/{action} ---

resource "aws_api_gateway_resource" "licenses" {
  rest_api_id = aws_api_gateway_rest_api.api.id
  parent_id   = aws_api_gateway_rest_api.api.root_resource_id
  path_part   = "licenses"
}

resource "aws_api_gateway_resource" "licenses_action" {
  rest_api_id = aws_api_gateway_rest_api.api.id
  parent_id   = aws_api_gateway_resource.licenses.id
  path_part   = "{action}"
}

resource "aws_api_gateway_method" "licenses_post" {
  rest_api_id   = aws_api_gateway_rest_api.api.id
  resource_id   = aws_api_gateway_resource.licenses_action.id
  http_method   = "POST"
  authorization = "NONE"
}

resource "aws_api_gateway_integration" "licenses_lambda" {
  rest_api_id             = aws_api_gateway_rest_api.api.id
  resource_id             = aws_api_gateway_resource.licenses_action.id
  http_method             = aws_api_gateway_method.licenses_post.http_method
  integration_http_method = "POST"
  type                    = "AWS_PROXY"
  uri                     = aws_lambda_function.license_manager.invoke_arn
}

# CORS OPTIONS for /licenses/{action}
resource "aws_api_gateway_method" "licenses_options" {
  rest_api_id   = aws_api_gateway_rest_api.api.id
  resource_id   = aws_api_gateway_resource.licenses_action.id
  http_method   = "OPTIONS"
  authorization = "NONE"
}

resource "aws_api_gateway_integration" "licenses_options" {
  rest_api_id = aws_api_gateway_rest_api.api.id
  resource_id = aws_api_gateway_resource.licenses_action.id
  http_method = aws_api_gateway_method.licenses_options.http_method
  type        = "MOCK"
  request_templates = {
    "application/json" = "{\"statusCode\": 200}"
  }
}

resource "aws_api_gateway_method_response" "licenses_options_200" {
  rest_api_id = aws_api_gateway_rest_api.api.id
  resource_id = aws_api_gateway_resource.licenses_action.id
  http_method = aws_api_gateway_method.licenses_options.http_method
  status_code = "200"
  response_parameters = {
    "method.response.header.Access-Control-Allow-Headers" = true
    "method.response.header.Access-Control-Allow-Methods" = true
    "method.response.header.Access-Control-Allow-Origin"  = true
  }
}

resource "aws_api_gateway_integration_response" "licenses_options_200" {
  rest_api_id = aws_api_gateway_rest_api.api.id
  resource_id = aws_api_gateway_resource.licenses_action.id
  http_method = aws_api_gateway_method.licenses_options.http_method
  status_code = aws_api_gateway_method_response.licenses_options_200.status_code
  response_parameters = {
    "method.response.header.Access-Control-Allow-Headers" = "'Content-Type'"
    "method.response.header.Access-Control-Allow-Methods" = "'POST,OPTIONS'"
    "method.response.header.Access-Control-Allow-Origin"  = "'*'"
  }
}

# --- Deployment + Stage ---

resource "aws_api_gateway_deployment" "deploy" {
  rest_api_id = aws_api_gateway_rest_api.api.id
  depends_on = [
    aws_api_gateway_integration.credentials_lambda,
    aws_api_gateway_integration.licenses_lambda,
    aws_api_gateway_integration.credentials_options,
    aws_api_gateway_integration.licenses_options,
  ]

  triggers = {
    redeployment = sha1(jsonencode([
      aws_api_gateway_resource.credentials.id,
      aws_api_gateway_resource.licenses_action.id,
      aws_api_gateway_method.credentials_post.id,
      aws_api_gateway_method.licenses_post.id,
    ]))
  }

  lifecycle {
    create_before_destroy = true
  }
}

resource "aws_api_gateway_stage" "v1" {
  rest_api_id   = aws_api_gateway_rest_api.api.id
  deployment_id = aws_api_gateway_deployment.deploy.id
  stage_name    = "v1"
}

# --- Lambda Permissions for API Gateway ---

resource "aws_lambda_permission" "apigw_credentials" {
  statement_id  = "AllowAPIGatewayCredentials"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.credential_proxy.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_api_gateway_rest_api.api.execution_arn}/*/POST/credentials"
}

resource "aws_lambda_permission" "apigw_licenses" {
  statement_id  = "AllowAPIGatewayLicenses"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.license_manager.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_api_gateway_rest_api.api.execution_arn}/*/POST/licenses/*"
}
