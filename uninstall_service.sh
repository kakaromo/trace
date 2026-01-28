#!/bin/bash
# Trace gRPC 서버 제거 스크립트

set -e

# 색상 정의
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 루트 권한 확인
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}❌ 이 스크립트는 root 권한이 필요합니다.${NC}"
    echo "sudo ./uninstall_service.sh 로 실행해주세요."
    exit 1
fi

echo -e "${YELLOW}🗑️  Trace gRPC 서버 제거 시작...${NC}"
echo ""

INSTALL_DIR="/opt/trace"
LOG_DIR="/var/log/trace"
SERVICE_FILE="trace-grpc.service"
SYSTEMD_DIR="/etc/systemd/system"

# 확인
read -p "정말로 Trace gRPC 서버를 제거하시겠습니까? (y/n): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "제거가 취소되었습니다."
    exit 0
fi
echo ""

# 1. 서비스 중지 및 비활성화
echo -e "${YELLOW}[1/5] 서비스 중지 및 비활성화...${NC}"
if systemctl is-active --quiet trace-grpc; then
    systemctl stop trace-grpc
    echo "✓ 서비스 중지 완료"
fi
if systemctl is-enabled --quiet trace-grpc 2>/dev/null; then
    systemctl disable trace-grpc
    echo "✓ 자동 시작 비활성화 완료"
fi
echo ""

# 2. systemd 서비스 파일 삭제
echo -e "${YELLOW}[2/5] systemd 서비스 파일 삭제...${NC}"
if [ -f "$SYSTEMD_DIR/$SERVICE_FILE" ]; then
    rm -f "$SYSTEMD_DIR/$SERVICE_FILE"
    systemctl daemon-reload
    echo "✓ 서비스 파일 삭제 완료"
fi
echo ""

# 3. 설치 디렉토리 삭제
echo -e "${YELLOW}[3/5] 설치 디렉토리 삭제...${NC}"
if [ -d "$INSTALL_DIR" ]; then
    rm -rf "$INSTALL_DIR"
    echo "✓ $INSTALL_DIR 삭제 완료"
fi
echo ""

# 4. 로그 디렉토리 삭제 (선택)
echo -e "${YELLOW}[4/5] 로그 디렉토리 삭제...${NC}"
read -p "로그 파일도 삭제하시겠습니까? (y/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    if [ -d "$LOG_DIR" ]; then
        rm -rf "$LOG_DIR"
        echo "✓ $LOG_DIR 삭제 완료"
    fi
else
    echo "✓ 로그 파일 보존"
fi
echo ""

# 5. 사용자 삭제 (선택)
echo -e "${YELLOW}[5/5] 사용자 삭제...${NC}"
read -p "사용자 'trace'를 삭제하시겠습니까? (y/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    if id -u trace >/dev/null 2>&1; then
        userdel trace
        echo "✓ 사용자 'trace' 삭제 완료"
    fi
else
    echo "✓ 사용자 'trace' 보존"
fi
echo ""

echo -e "${GREEN}✅ 제거가 완료되었습니다!${NC}"
