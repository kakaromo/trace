use flate2::read::GzDecoder;
use sevenz_rust::SevenZReader;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use tar::Archive;
use xz2::read::XzDecoder;
use zip::ZipArchive;

/// 압축 파일 형식
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionFormat {
    Tar,
    TarGz,
    TarXz,
    Zip,
    SevenZ,
    None,
}

impl CompressionFormat {
    /// 파일 경로에서 압축 형식 감지
    pub fn from_path(path: &Path) -> Self {
        let path_str = path.to_string_lossy().to_lowercase();
        
        if path_str.ends_with(".tar.gz") || path_str.ends_with(".tgz") {
            CompressionFormat::TarGz
        } else if path_str.ends_with(".tar.xz") || path_str.ends_with(".txz") {
            CompressionFormat::TarXz
        } else if path_str.ends_with(".tar") {
            CompressionFormat::Tar
        } else if path_str.ends_with(".zip") {
            CompressionFormat::Zip
        } else if path_str.ends_with(".7z") {
            CompressionFormat::SevenZ
        } else {
            CompressionFormat::None
        }
    }
}

/// 압축 파일을 해제하고 첫 번째 CSV/로그 파일의 경로를 반환
pub fn extract_and_find_log(
    compressed_file: &Path,
    output_dir: &Path,
) -> io::Result<PathBuf> {
    let format = CompressionFormat::from_path(compressed_file);
    
    if format == CompressionFormat::None {
        // 압축되지 않은 파일은 그대로 반환
        return Ok(compressed_file.to_path_buf());
    }
    
    println!("Detected compression format: {:?}", format);
    println!("Extracting {} to {}", compressed_file.display(), output_dir.display());
    
    // 출력 디렉토리 생성
    fs::create_dir_all(output_dir)?;
    
    // 압축 해제
    match format {
        CompressionFormat::Tar => extract_tar(compressed_file, output_dir)?,
        CompressionFormat::TarGz => extract_tar_gz(compressed_file, output_dir)?,
        CompressionFormat::TarXz => extract_tar_xz(compressed_file, output_dir)?,
        CompressionFormat::Zip => extract_zip(compressed_file, output_dir)?,
        CompressionFormat::SevenZ => extract_7z(compressed_file, output_dir)?,
        CompressionFormat::None => unreachable!(),
    }
    
    println!("Extraction completed, searching for log files...");
    
    // 압축 해제된 파일 중 CSV/로그 파일 찾기
    find_log_file(output_dir)
}

/// tar 파일 압축 해제
fn extract_tar(tar_file: &Path, output_dir: &Path) -> io::Result<()> {
    let file = File::open(tar_file)?;
    let mut archive = Archive::new(file);
    archive.unpack(output_dir)?;
    Ok(())
}

/// tar.gz 파일 압축 해제
fn extract_tar_gz(tar_gz_file: &Path, output_dir: &Path) -> io::Result<()> {
    let file = File::open(tar_gz_file)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive.unpack(output_dir)?;
    Ok(())
}

/// tar.xz 파일 압축 해제
fn extract_tar_xz(tar_xz_file: &Path, output_dir: &Path) -> io::Result<()> {
    let file = File::open(tar_xz_file)?;
    let decoder = XzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive.unpack(output_dir)?;
    Ok(())
}

/// zip 파일 압축 해제
fn extract_zip(zip_file: &Path, output_dir: &Path) -> io::Result<()> {
    let file = File::open(zip_file)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        
        let outpath = match file.enclosed_name() {
            Some(path) => output_dir.join(path),
            None => continue,
        };
        
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }
    
    Ok(())
}

/// 7z 파일 압축 해제
fn extract_7z(sevenz_file: &Path, output_dir: &Path) -> io::Result<()> {
    let mut reader = SevenZReader::open(sevenz_file, "".into())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Failed to open 7z file: {:?}", e)))?;
    
    reader.for_each_entries(|entry, reader| {
        let entry_path = output_dir.join(entry.name());
        
        if entry.is_directory() {
            fs::create_dir_all(&entry_path)?;
        } else {
            if let Some(parent) = entry_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&entry_path)?;
            io::copy(reader, &mut outfile)?;
        }
        
        Ok(true) // continue processing
    }).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Failed to extract 7z: {:?}", e)))?;
    
    Ok(())
}

/// 디렉토리에서 CSV 또는 로그 파일 찾기
fn find_log_file(dir: &Path) -> io::Result<PathBuf> {
    let mut log_files = Vec::new();
    
    // 재귀적으로 파일 검색
    find_log_files_recursive(dir, &mut log_files)?;
    
    if log_files.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("No CSV or log files found in extracted directory: {}", dir.display()),
        ));
    }
    
    // 첫 번째 로그 파일 반환 (여러 파일이 있으면 가장 큰 파일 선택)
    log_files.sort_by_key(|path| {
        fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    });
    
    let selected_file = log_files.last().unwrap().clone();
    println!("Found log file: {} ({} bytes)", 
        selected_file.display(),
        fs::metadata(&selected_file)?.len()
    );
    
    Ok(selected_file)
}

/// 재귀적으로 로그 파일 찾기
fn find_log_files_recursive(dir: &Path, log_files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            find_log_files_recursive(&path, log_files)?;
        } else if path.is_file() {
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();
            
            // CSV 또는 로그 파일 확장자 확인
            if file_name.ends_with(".csv") 
                || file_name.ends_with(".log")
                || file_name.ends_with(".txt")
                || file_name.contains("trace")
                || file_name.contains("blktrace") {
                log_files.push(path);
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compression_format_detection() {
        assert_eq!(
            CompressionFormat::from_path(Path::new("file.tar.gz")),
            CompressionFormat::TarGz
        );
        assert_eq!(
            CompressionFormat::from_path(Path::new("file.tgz")),
            CompressionFormat::TarGz
        );
        assert_eq!(
            CompressionFormat::from_path(Path::new("file.tar.xz")),
            CompressionFormat::TarXz
        );
        assert_eq!(
            CompressionFormat::from_path(Path::new("file.tar")),
            CompressionFormat::Tar
        );
        assert_eq!(
            CompressionFormat::from_path(Path::new("file.zip")),
            CompressionFormat::Zip
        );
        assert_eq!(
            CompressionFormat::from_path(Path::new("file.7z")),
            CompressionFormat::SevenZ
        );
        assert_eq!(
            CompressionFormat::from_path(Path::new("file.csv")),
            CompressionFormat::None
        );
    }
}
