use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::sync::Once;
use std::sync::OnceLock;

// 전역 로거 인스턴스를 저장할 정적 변수
static LOGGER: OnceLock<Mutex<Option<File>>> = OnceLock::new();
static INIT: Once = Once::new();

pub struct Logger;

impl Logger {
    pub fn init(output_path: &str) {
        INIT.call_once(|| {
            // 경로가 슬래시로 끝나는지 확인
            let output_path = output_path.trim_end_matches('/');

            // 출력 파일 경로에서 디렉토리 부분 추출
            let path = Path::new(output_path);

            // 경로 구성 방식에 따라 적절한 로그 파일 경로 생성
            let log_path = if path.is_dir() || output_path.ends_with('/') {
                // 디렉토리 경로인 경우
                let dir = path;
                let log_filename = "result.log";
                dir.join(log_filename)
            } else {
                // 파일 접두사인 경우
                let dir = path.parent().unwrap_or_else(|| Path::new("."));
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("output");

                dir.join(format!("{file_name}_result.log"))
            };

            // 디렉토리가 없으면 생성
            let log_dir = log_path.parent().unwrap_or_else(|| Path::new("."));
            if !log_dir.exists() {
                if let Err(e) = fs::create_dir_all(log_dir) {
                    eprintln!("로그 디렉토리를 생성할 수 없습니다: {e}");
                    LOGGER.get_or_init(|| Mutex::new(None));
                    return;
                }
            }

            // 로그 파일 열기
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&log_path);

            match file {
                Ok(file) => {
                    println!("로그를 '{}'에 저장합니다.", log_path.display());
                    LOGGER.get_or_init(|| Mutex::new(Some(file)));
                }
                Err(e) => {
                    eprintln!("로그 파일을 열 수 없습니다: {e}");
                    LOGGER.get_or_init(|| Mutex::new(None));
                }
            }
        });
    }

    pub fn log(message: &str) {
        // 콘솔에 출력
        println!("{message}");

        // 파일에도 동일한 내용 기록
        if let Some(logger) = LOGGER.get() {
            if let Ok(mut file_guard) = logger.lock() {
                if let Some(file) = file_guard.as_mut() {
                    if let Err(e) = writeln!(file, "{message}") {
                        eprintln!("로그 파일 쓰기 실패: {e}");
                    }
                }
            }
        }
    }

    pub fn log_error(message: &str) {
        // 콘솔에 에러 출력
        eprintln!("{message}");

        // 파일에도 동일한 내용 기록
        if let Some(logger) = LOGGER.get() {
            if let Ok(mut file_guard) = logger.lock() {
                if let Some(file) = file_guard.as_mut() {
                    if let Err(e) = writeln!(file, "ERROR: {message}") {
                        eprintln!("에러 로그 파일 쓰기 실패: {e}");
                    }
                }
            }
        }
    }

    pub fn log_fmt(args: std::fmt::Arguments<'_>) {
        // 콘솔에 출력
        println!("{args}");

        // 파일에도 동일한 내용 기록
        if let Some(logger) = LOGGER.get() {
            if let Ok(mut file_guard) = logger.lock() {
                if let Some(file) = file_guard.as_mut() {
                    if let Err(e) = writeln!(file, "{args}") {
                        eprintln!("로그 파일 쓰기 실패: {e}");
                    }
                }
            }
        }
    }

    pub fn flush() -> std::io::Result<()> {
        if let Some(logger) = LOGGER.get() {
            if let Ok(mut file_guard) = logger.lock() {
                if let Some(file) = file_guard.as_mut() {
                    file.flush()?;
                }
            }
        }
        Ok(())
    }
}

// 매크로 정의
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        $crate::utils::Logger::log(&message);
    }};
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        $crate::utils::Logger::log_error(&message);
    }};
}
