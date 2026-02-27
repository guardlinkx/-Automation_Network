variable "aws_region" {
  description = "AWS region for all resources"
  type        = string
  default     = "us-east-1"
}

variable "bucket_name" {
  description = "S3 bucket name for subscriber backups (must be globally unique)"
  type        = string
  default     = "driveguard-backups"
}

variable "credential_duration" {
  description = "STS credential lifetime in seconds"
  type        = number
  default     = 3600
}

variable "firebase_credentials_file" {
  description = "Path to the Firebase service account JSON file"
  type        = string
}

variable "firebase_project_id" {
  description = "Firebase/GCP project ID"
  type        = string
}

variable "lambda_memory" {
  description = "Lambda memory in MB (extra for firebase-admin SDK)"
  type        = number
  default     = 256
}
