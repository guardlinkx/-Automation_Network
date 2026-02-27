output "api_endpoint" {
  description = "Base API Gateway URL"
  value       = aws_api_gateway_stage.v1.invoke_url
}

output "credential_proxy_url" {
  description = "Full URL for the credential proxy endpoint (set in cloud_backend.py)"
  value       = "${aws_api_gateway_stage.v1.invoke_url}/credentials"
}

output "license_api_base_url" {
  description = "Base URL for license management endpoints (set in license_manager.py)"
  value       = "${aws_api_gateway_stage.v1.invoke_url}/licenses"
}

output "bucket_name" {
  description = "S3 bucket for subscriber backups"
  value       = aws_s3_bucket.backups.id
}

output "upload_role_arn" {
  description = "IAM role ARN for scoped S3 access"
  value       = aws_iam_role.s3_upload.arn
}
