use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use std::fs::File;
use std::io::{Read, Write};
use tokio::runtime::Runtime;

/// MinIO 연결 설정
#[derive(Debug, Clone)]
pub struct MinioConfig {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    pub region: String,
}

impl MinioConfig {
    /// 환경 변수에서 MinIO 설정 읽기
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(MinioConfig {
            endpoint: std::env::var("MINIO_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:9000".to_string()),
            access_key: std::env::var("MINIO_ACCESS_KEY")?,
            secret_key: std::env::var("MINIO_SECRET_KEY")?,
            bucket: std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "trace".to_string()),
            region: std::env::var("MINIO_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
        })
    }

    /// 직접 설정 생성
    pub fn new(
        endpoint: String,
        access_key: String,
        secret_key: String,
        bucket: String,
    ) -> Self {
        MinioConfig {
            endpoint,
            access_key,
            secret_key,
            bucket,
            region: "us-east-1".to_string(),
        }
    }
}

/// MinIO 클라이언트
pub struct MinioClient {
    bucket: Bucket,
    runtime: Runtime,
}

impl MinioClient {
    /// MinIO 클라이언트 생성
    pub fn new(config: &MinioConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let credentials = Credentials::new(
            Some(&config.access_key),
            Some(&config.secret_key),
            None,
            None,
            None,
        )?;

        let region = Region::Custom {
            region: config.region.clone(),
            endpoint: config.endpoint.clone(),
        };

        let bucket = Bucket::new(&config.bucket, region, credentials)?
            .with_path_style(); // MinIO는 path-style 사용

        let runtime = Runtime::new()?;

        Ok(MinioClient { bucket: *bucket, runtime })
    }

    /// MinIO에서 파일 다운로드 (동기)
    pub fn download_file(
        &self,
        remote_path: &str,
        local_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Downloading from MinIO: {remote_path} → {local_path}");

        // MinIO에서 파일 가져오기 (비동기를 동기로 변환)
        let response = self.runtime.block_on(async {
            self.bucket.get_object(remote_path).await
        })?;

        // 로컬 파일로 저장
        let mut file = File::create(local_path)?;
        file.write_all(&response.bytes())?;

        println!("Download completed: {} bytes", response.bytes().len());
        Ok(())
    }

    /// MinIO에 파일 업로드 (동기)
    pub fn upload_file(
        &self,
        local_path: &str,
        remote_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Uploading to MinIO: {local_path} → {remote_path}");

        // 로컬 파일 읽기
        let mut file = File::open(local_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        // MinIO에 업로드 (비동기를 동기로 변환)
        let response = self.runtime.block_on(async {
            self.bucket.put_object(remote_path, &buffer).await
        })?;

        println!("Upload completed: {} (status code: {})", 
            remote_path, response.status_code());
        Ok(())
    }

    /// MinIO에서 파일 존재 여부 확인
    pub fn file_exists(&self, remote_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let result = self.runtime.block_on(async {
            self.bucket.head_object(remote_path).await
        });
        
        match result {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// MinIO에서 파일 삭제
    pub fn delete_file(&self, remote_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Deleting from MinIO: {remote_path}");
        self.runtime.block_on(async {
            self.bucket.delete_object(remote_path).await
        })?;
        println!("Delete completed: {remote_path}");
        Ok(())
    }

    /// MinIO 버킷의 파일 목록 조회
    pub fn list_files(&self, prefix: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let results = self.runtime.block_on(async {
            self.bucket.list(prefix.to_string(), None).await
        })?;
        
        let mut files = Vec::new();
        for result in results {
            for content in result.contents {
                files.push(content.key);
            }
        }
        
        Ok(files)
    }
}

/// MinIO에서 로그 파일을 읽어서 로컬에 임시 저장
pub fn download_log_from_minio(
    config: &MinioConfig,
    remote_log_path: &str,
    temp_local_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = MinioClient::new(config)?;
    client.download_file(remote_log_path, temp_local_path)?;
    Ok(())
}

/// Parquet 파일을 MinIO에 업로드
pub fn upload_parquet_to_minio(
    config: &MinioConfig,
    local_parquet_path: &str,
    remote_parquet_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = MinioClient::new(config)?;
    client.upload_file(local_parquet_path, remote_parquet_path)?;
    Ok(())
}

/// MinIO에서 Parquet 파일을 다운로드
pub fn download_parquet_from_minio(
    config: &MinioConfig,
    remote_parquet_path: &str,
    local_parquet_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = MinioClient::new(config)?;
    client.download_file(remote_parquet_path, local_parquet_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // MinIO 서버가 필요하므로 기본적으로는 무시
    fn test_minio_operations() {
        let config = MinioConfig::new(
            "http://localhost:9000".to_string(),
            "admin".to_string(),
            "tka123tjd!".to_string(),
            "trace".to_string(),
        );

        let client = MinioClient::new(&config).unwrap();
        
        // 파일 목록 조회 테스트
        let files = client.list_files("").unwrap();
        println!("Files in bucket: {:?}", files);
    }
}
