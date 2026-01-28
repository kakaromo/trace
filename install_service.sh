#!/bin/bash
# Trace gRPC 서버 설치 스크립트

set -e

# 색상 정의
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 루트 권한 확인
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}❌ 이 스크립트는 root 권한이 필요합니다.${NC}"
    echo "sudo ./install_service.sh 로 실행해주세요."
    exit 1
fi

echo -e "${GREEN}🚀 Trace gRPC 서버 설치 시작...${NC}"
echo ""

# 설치 디렉토리
INSTALL_DIR="/opt/trace"
BIN_DIR="$INSTALL_DIR/bin"
CONFIG_DIR="$INSTALL_DIR/config"
LOG_DIR="/var/log/trace"
SERVICE_FILE="trace-grpc.service"
SYSTEMD_DIR="/etc/systemd/system"

# 1. 사용자 및 그룹 생성
echo -e "${YELLOW}[1/6] 사용자 및 그룹 생성...${NC}"
if ! id -u trace >/dev/null 2>&1; then
    useradd -r -s /bin/false -d $INSTALL_DIR -c "Trace gRPC Server" trace
    echo "✓ 사용자 'trace' 생성 완료"
else
    echo "✓ 사용자 'trace' 이미 존재"
fi
echo ""

# 2. 디렉토리 생성
echo -e "${YELLOW}[2/6] 디렉토리 생성...${NC}"
mkdir -p $BIN_DIR
mkdir -p $CONFIG_DIR
mkdir -p $LOG_DIR
mkdir -p /tmp/trace
echo "✓ 디렉토리 생성 완료"
echo ""

# 3. 바이너리 복사
echo -e "${YELLOW}[3/6] 바이너리 복사...${NC}"
if [ -f "target/release/trace" ]; then
    cp target/release/trace $BIN_DIR/
    chmod +x $BIN_DIR/trace
    echo "✓ 바이너리 복사 완료: $BIN_DIR/trace"
else
    echo -e "${RED}❌ target/release/trace 파일을 찾을 수 없습니다.${NC}"
    echo "먼저 'cargo build --release'를 실행해주세요."
    exit 1
fi
echo ""

# 4. 환경 설정 파일 생성
echo -e "${YELLOW}[4/6] 환경 설정 파일 생성...${NC}"
cat > $CONFIG_DIR/grpc.env << EOF
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
EOF
echo "✓ 설정 파일 생성 완료: $CONFIG_DIR/grpc.env"
echo ""

# 5. systemd 서비스 파일 복사
echo -e "${YELLOW}[5/6] systemd 서비스 등록...${NC}"
if [ -f "$SERVICE_FILE" ]; then
    cp $SERVICE_FILE $SYSTEMD_DIR/
    systemctl daemon-reload
    echo "✓ 서비스 파일 복사 완료: $SYSTEMD_DIR/$SERVICE_FILE"
else
    echo -e "${RED}❌ $SERVICE_FILE 파일을 찾을 수 없습니다.${NC}"
    exit 1
fi
echo ""

# 6. 권한 설정
echo -e "${YELLOW}[6/6] 권한 설정...${NC}"
chown -R trace:trace $INSTALL_DIR
chown -R trace:trace $LOG_DIR
chown trace:trace /tmp/trace
chmod 755 $BIN_DIR/trace
chmod 644 $CONFIG_DIR/grpc.env
echo "✓ 권한 설정 완료"
echo ""

# 서비스 활성화 및 시작 여부 확인
echo -e "${GREEN}✅ 설치가 완료되었습니다!${NC}"
echo ""
echo "다음 명령어로 서비스를 관리할 수 있습니다:"
echo ""
echo "  서비스 시작:     sudo systemctl start trace-grpc"
echo "  서비스 중지:     sudo systemctl stop trace-grpc"
echo "  서비스 재시작:   sudo systemctl restart trace-grpc"
echo "  서비스 상태:     sudo systemctl status trace-grpc"
echo "  자동 시작 설정:  sudo systemctl enable trace-grpc"
echo "  로그 확인:       sudo journalctl -u trace-grpc -f"
echo ""
echo -e "${YELLOW}⚠️  주의사항:${NC}"
echo "  1. $CONFIG_DIR/grpc.env 파일에서 MinIO 설정을 확인하세요."
echo "  2. 서비스 시작 전에 MinIO가 실행 중인지 확인하세요."
echo ""

# 서비스 시작 여부 확인
read -p "지금 서비스를 시작하시겠습니까? (y/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    systemctl start trace-grpc
    systemctl enable trace-grpc
    echo ""
    echo -e "${GREEN}✅ 서비스가 시작되었고 자동 시작이 설정되었습니다.${NC}"
    echo ""
    sleep 2
    systemctl status trace-grpc --no-pager
fi
