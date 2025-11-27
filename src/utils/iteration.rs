use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Iteration별 출력 디렉토리 관리자
pub struct IterationOutputManager {
    base_dir: PathBuf,
}

impl IterationOutputManager {
    /// 새로운 IterationOutputManager 생성
    ///
    /// # Arguments
    /// * `output_prefix` - 출력 파일 접두사 (예: "fio_result")
    pub fn new(output_prefix: &str) -> io::Result<Self> {
        let base_dir = PathBuf::from(output_prefix);

        // 베이스 디렉토리 생성
        if !base_dir.exists() {
            fs::create_dir_all(&base_dir)?;
        }

        Ok(Self { base_dir })
    }

    /// Iteration 번호에 해당하는 디렉토리 경로 반환
    ///
    /// # Arguments
    /// * `iteration` - Iteration 번호 (1, 2, 3, ...)
    pub fn get_iteration_dir(&self, iteration: usize) -> PathBuf {
        self.base_dir.join(iteration.to_string())
    }

    /// Iteration 디렉토리 생성
    ///
    /// # Arguments
    /// * `iteration` - Iteration 번호
    pub fn create_iteration_dir(&self, iteration: usize) -> io::Result<PathBuf> {
        let iter_dir = self.get_iteration_dir(iteration);

        if !iter_dir.exists() {
            fs::create_dir_all(&iter_dir)?;
        }

        Ok(iter_dir)
    }

    /// Iteration 디렉토리에 파일 경로 생성
    ///
    /// # Arguments
    /// * `iteration` - Iteration 번호
    /// * `filename` - 파일명 (예: "ufs_trace.parquet")
    pub fn get_file_path(&self, iteration: usize, filename: &str) -> io::Result<PathBuf> {
        let iter_dir = self.create_iteration_dir(iteration)?;
        Ok(iter_dir.join(filename))
    }

    /// 모든 iteration 디렉토리 나열
    pub fn list_iterations(&self) -> io::Result<Vec<usize>> {
        let mut iterations = Vec::new();

        if self.base_dir.exists() {
            for entry in fs::read_dir(&self.base_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    if let Some(dirname) = path.file_name() {
                        if let Some(dirname_str) = dirname.to_str() {
                            if let Ok(iter_num) = dirname_str.parse::<usize>() {
                                iterations.push(iter_num);
                            }
                        }
                    }
                }
            }
        }

        iterations.sort();
        Ok(iterations)
    }

    /// 베이스 디렉토리 경로 반환
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// 특정 iteration 디렉토리가 존재하는지 확인
    pub fn iteration_exists(&self, iteration: usize) -> bool {
        self.get_iteration_dir(iteration).exists()
    }

    /// 특정 iteration 디렉토리의 파일 목록 반환
    pub fn list_files(&self, iteration: usize) -> io::Result<Vec<String>> {
        let iter_dir = self.get_iteration_dir(iteration);
        let mut files = Vec::new();

        if iter_dir.exists() {
            for entry in fs::read_dir(iter_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    if let Some(filename) = path.file_name() {
                        if let Some(filename_str) = filename.to_str() {
                            files.push(filename_str.to_string());
                        }
                    }
                }
            }
        }

        files.sort();
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_iteration_manager() {
        let temp_dir = "test_output_temp";

        // 테스트용 매니저 생성
        let manager = IterationOutputManager::new(temp_dir).unwrap();

        // Iteration 디렉토리 생성
        let iter1_dir = manager.create_iteration_dir(1).unwrap();
        let iter2_dir = manager.create_iteration_dir(2).unwrap();

        assert!(iter1_dir.exists());
        assert!(iter2_dir.exists());

        // 파일 경로 생성
        let file_path = manager.get_file_path(1, "test.parquet").unwrap();
        assert!(file_path.to_str().unwrap().contains("test.parquet"));

        // Iteration 목록 확인
        let iterations = manager.list_iterations().unwrap();
        assert_eq!(iterations, vec![1, 2]);

        // 정리
        fs::remove_dir_all(temp_dir).ok();
    }
}
