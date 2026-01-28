# Systemd 서비스 등록 가이드

이 문서는 Trace gRPC 서버를 Linux 시스템에 systemd 서비스로 등록하는 방법을 설명합니다.

## 파일 설명

- **trace-grpc.service**: systemd unit 파일
- **install_service.sh**: 자동 설치 스크립트
- **uninstall_service.sh**: 제거 스크립트

## 빠른 설치

```bash
# 1. 릴리즈 빌드
cargo build --release

# 2. 설치 스크립트 실행 (root 권한 필요)
sudo ./install_service.sh
```

## 설치 내용

설치 스크립트는 다음 작업을 수행합니다:

1. **사용자 생성**: `trace` 사용자 및 그룹 생성
2. **디렉토리 생성**:
   - `/opt/trace/bin`: 바이너리 파일
   - `/opt/trace/config`: 설정 파일
   - `/var/log/trace`: 로그 파일
   - `/tmp/trace`: 임시 파일
3. **바이너리 복사**: `target/release/trace` → `/opt/trace/bin/trace`
4. **설정 파일 생성**: `/opt/trace/config/grpc.env`
5. **systemd 서비스 등록**: `/etc/systemd/system/trace-grpc.service`
6. **권한 설정**: 적절한 소유자 및 권한 설정

## 서비스 관리

### 서비스 시작
```bash
sudo systemctl start trace-grpc
```

### 서비스 중지
```bash
sudo systemctl stop trace-grpc
```

### 서비스 재시작
```bash
sudo systemctl restart trace-grpc
```

### 서비스 상태 확인
```bash
sudo systemctl status trace-grpc
```

### 자동 시작 설정
```bash
sudo systemctl enable trace-grpc
```

### 자동 시작 해제
```bash
sudo systemctl disable trace-grpc
```

## 로그 확인

### 실시간 로그 확인
```bash
sudo journalctl -u trace-grpc -f
```

### 최근 로그 확인
```bash
sudo journalctl -u trace-grpc -n 100
```

### 특정 시간대 로그 확인
```bash
sudo journalctl -u trace-grpc --since "2024-01-01 00:00:00"
```

## 설정 변경

### MinIO 설정 변경
```bash
sudo nano /opt/trace/config/grpc.env
```

설정 파일 예시:
```bash
# MinIO 설정
MINIO_ENDPOINT=http://localhost:9000
MINIO_ACCESS_KEY=minioadmin
MINIO_SECRET_KEY=minioadmin
MINIO_BUCKET=trace
MINIO_REGION=us-east-1

# gRPC 설정
GRPC_PORT=50051
GRPC_ADDRESS=0.0.0.0

# 로그 설정
RUST_LOG=info
```

### 서비스 파일 변경
```bash
sudo nano /etc/systemd/system/trace-grpc.service
```

변경 후 재시작:
```bash
sudo systemctl daemon-reload
sudo systemctl restart trace-grpc
```

## 제거

```bash
sudo ./uninstall_service.sh
```

제거 스크립트는 다음을 수행합니다:
1. 서비스 중지 및 비활성화
2. systemd 서비스 파일 삭제
3. 설치 디렉토리 삭제
4. 로그 디렉토리 삭제 (선택)
5. 사용자 삭제 (선택)

## 수동 설치 (고급)

자동 스크립트를 사용하지 않고 수동으로 설치하려면:

```bash
# 1. 사용자 생성
sudo useradd -r -s /bin/false -d /opt/trace -c "Trace gRPC Server" trace

# 2. 디렉토리 생성
sudo mkdir -p /opt/trace/bin
sudo mkdir -p /opt/trace/config
sudo mkdir -p /var/log/trace
sudo mkdir -p /tmp/trace

# 3. 바이너리 복사
sudo cp target/release/trace /opt/trace/bin/
sudo chmod +x /opt/trace/bin/trace

# 4. 설정 파일 생성
sudo nano /opt/trace/config/grpc.env
# (위의 설정 내용 입력)

# 5. 서비스 파일 복사
sudo cp trace-grpc.service /etc/systemd/system/
sudo systemctl daemon-reload

# 6. 권한 설정
sudo chown -R trace:trace /opt/trace
sudo chown -R trace:trace /var/log/trace
sudo chown trace:trace /tmp/trace

# 7. 서비스 시작
sudo systemctl start trace-grpc
sudo systemctl enable trace-grpc
```

## 문제 해결

### 서비스가 시작되지 않을 때
```bash
# 상세 로그 확인
sudo journalctl -u trace-grpc -xe

# 서비스 상태 확인
sudo systemctl status trace-grpc

# 바이너리 실행 테스트
sudo -u trace /opt/trace/bin/trace --grpc-server
```

### MinIO 연결 실패
1. MinIO가 실행 중인지 확인
2. `/opt/trace/config/grpc.env`의 MINIO_ENDPOINT 확인
3. 방화벽 설정 확인

### 포트 충돌
```bash
# 포트 사용 확인
sudo netstat -tulpn | grep 50051
# 또는
sudo ss -tulpn | grep 50051
```

설정 파일에서 GRPC_PORT를 변경하고 서비스 재시작

## 성능 튜닝

### 리소스 제한 조정
서비스 파일에서 리소스 제한을 조정할 수 있습니다:

```ini
[Service]
LimitNOFILE=65536      # 파일 디스크립터 제한
LimitNPROC=4096        # 프로세스 제한
LimitMEMLOCK=infinity  # 메모리 잠금 제한
```

### 로그 레벨 조정
```bash
# /opt/trace/config/grpc.env에서
RUST_LOG=debug  # 상세 로그
RUST_LOG=info   # 일반 로그
RUST_LOG=warn   # 경고만
RUST_LOG=error  # 에러만
```

## 보안 권장사항

1. **별도 사용자 실행**: 서비스는 전용 `trace` 사용자로 실행됩니다.
2. **최소 권한**: 필요한 디렉토리에만 접근 권한을 부여합니다.
3. **방화벽 설정**: 필요한 포트(50051)만 열어둡니다.
4. **MinIO 인증**: 기본 인증 정보를 변경하세요.

```bash
# 방화벽 설정 (firewalld)
sudo firewall-cmd --permanent --add-port=50051/tcp
sudo firewall-cmd --reload

# 방화벽 설정 (ufw)
sudo ufw allow 50051/tcp
```
