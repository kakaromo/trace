class RealTimeDashboard {
    constructor() {
        this.charts = {};
        this.eCharts = {};
        this.isConnected = false;
        this.updateInterval = null;
        this.pollingInterval = 2000; // 2초마다 폴링
        
        this.initializePolling();
        this.initializeCharts();
        this.initializeECharts();
        this.bindEvents();
    }

    initializePolling() {
        console.log('REST API 폴링 시작');
        
        // 기존 폴링 중단
        if (this.updateInterval) {
            clearInterval(this.updateInterval);
        }
        
        this.updateConnectionStatus(true, '연결 중...');
        
        // 즉시 한 번 업데이트
        this.fetchDashboardData();
        
        // 정기적으로 업데이트
        this.updateInterval = setInterval(() => {
            this.fetchDashboardData();
        }, this.pollingInterval);
    }

    async fetchDashboardData() {
        try {
            const response = await fetch('/api/dashboard');
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            const data = await response.json();
            
            // 연결 상태 업데이트
            if (!this.isConnected) {
                this.isConnected = true;
                this.updateConnectionStatus(true);
            }
            
            this.handleMessage({ message_type: 'dashboard_update', data: data });
        } catch (error) {
            console.error('API 요청 오류:', error);
            this.isConnected = false;
            this.updateConnectionStatus(false, 'API 오류');
        }
    }

    attemptReconnect() {
        console.log('REST API 재연결 시도');
        this.isConnected = false;
        this.updateConnectionStatus(false, '재연결 중...');
        
        // 잠시 후 다시 폴링 시작
        setTimeout(() => {
            this.initializePolling();
        }, 2000);
    }

    updateConnectionStatus(isConnected, message = null) {
        const statusDot = document.getElementById('connection-status');
        const statusText = document.getElementById('connection-text');
        
        if (isConnected) {
            statusDot.className = 'status-dot online';
            statusText.textContent = '연결됨';
            statusText.style.color = '#10b981';
        } else {
            statusDot.className = 'status-dot offline';
            statusText.textContent = message || '연결 끊김';
            statusText.style.color = '#ef4444';
        }
    }

    handleMessage(message) {
        switch (message.message_type) {
            case 'dashboard_update':
                this.updateDashboard(message.data);
                break;
            case 'alert':
                this.handleAlert(message.data);
                break;
            case 'trend_update':
                this.updateTrends(message.data);
                break;
            case 'anomaly_detected':
                this.handleAnomaly(message.data);
                break;
            default:
                console.log('알 수 없는 메시지 타입:', message.message_type);
        }
    }

    updateDashboard(data) {
        this.updateStats(data);
        this.updateCharts(data);
        this.updateAlerts(data.alerts);
        this.updateTrends(data.trends);
        this.updateAnomalies(data.anomalies);
        this.updateRecentEntries(data.recent_entries);
        this.updateECharts(data); // ECharts 업데이트 추가
    }

    updateStats(data) {
        document.getElementById('total-entries').textContent = this.formatNumber(data.total_entries);
        document.getElementById('processing-rate').textContent = `${data.processing_rate.toFixed(1)}/초`;
        document.getElementById('avg-latency').textContent = `${data.avg_latency.toFixed(2)}ms`;
        document.getElementById('max-latency').textContent = `${data.max_latency.toFixed(2)}ms`;
        document.getElementById('block-count').textContent = this.formatNumber(data.block_count);
        document.getElementById('ufs-count').textContent = this.formatNumber(data.ufs_count);
        document.getElementById('custom-count').textContent = this.formatNumber(data.custom_count);
    }

    updateCharts(data) {
        // 처리율 차트 업데이트
        if (this.charts.processingRate) {
            const now = new Date();
            this.charts.processingRate.data.labels.push(now.toLocaleTimeString());
            this.charts.processingRate.data.datasets[0].data.push(data.processing_rate);
            
            // 최대 50개 데이터 포인트 유지
            if (this.charts.processingRate.data.labels.length > 50) {
                this.charts.processingRate.data.labels.shift();
                this.charts.processingRate.data.datasets[0].data.shift();
            }
            
            this.charts.processingRate.update('none');
        }

        // 레이턴시 차트 업데이트
        if (this.charts.latency) {
            this.charts.latency.data.datasets[0].data = [
                data.avg_latency,
                data.max_latency,
                data.min_latency
            ];
            this.charts.latency.update('none');
        }

        // 트레이스 타입 차트 업데이트
        if (this.charts.traceType) {
            this.charts.traceType.data.datasets[0].data = [
                data.block_count,
                data.ufs_count,
                data.custom_count
            ];
            this.charts.traceType.update('none');
        }
    }

    updateAlerts(alerts) {
        const container = document.getElementById('alerts-container');
        
        if (!alerts || alerts.length === 0) {
            container.innerHTML = '<div class="no-alerts">현재 알림이 없습니다.</div>';
            return;
        }

        container.innerHTML = alerts.map(alert => `
            <div class="alert-item ${alert.severity} fade-in">
                <div class="alert-message">${alert.message}</div>
                <div class="alert-timestamp">${this.formatTimestamp(alert.timestamp)}</div>
            </div>
        `).join('');
    }

    updateTrends(trends) {
        const container = document.getElementById('trends-container');
        
        if (!trends || trends.length === 0) {
            container.innerHTML = '<div class="no-trends">트렌드 데이터를 수집 중입니다...</div>';
            return;
        }

        container.innerHTML = trends.map(trend => `
            <div class="trend-item fade-in">
                <div class="trend-header">
                    <span class="trend-direction">${this.getTrendIcon(trend.direction)}</span>
                    <span class="trend-metric">${trend.metric}</span>
                </div>
                <div class="trend-details">
                    변화율: ${trend.change_rate.toFixed(2)} | 신뢰도: ${trend.confidence.toFixed(1)}%
                </div>
            </div>
        `).join('');
    }

    updateAnomalies(anomalies) {
        const container = document.getElementById('anomalies-container');
        
        if (!anomalies || anomalies.length === 0) {
            container.innerHTML = '<div class="no-anomalies">이상 징후가 감지되지 않았습니다.</div>';
            return;
        }

        container.innerHTML = anomalies.map(anomaly => `
            <div class="anomaly-item fade-in">
                <div class="anomaly-info">
                    <div class="anomaly-metric">${anomaly.metric}</div>
                    <div class="anomaly-details">
                        값: ${anomaly.value.toFixed(2)} | 
                        ${this.formatTimestamp(anomaly.timestamp)}
                    </div>
                </div>
                <div class="anomaly-score">Z: ${anomaly.z_score.toFixed(2)}</div>
            </div>
        `).join('');
    }

    updateRecentEntries(entries) {
        const container = document.getElementById('recent-entries-container');
        
        if (!entries || entries.length === 0) {
            container.innerHTML = '<div class="no-entries">최근 엔트리를 불러오는 중...</div>';
            return;
        }

        container.innerHTML = entries.map(entry => `
            <div class="entry-item fade-in">
                <div class="entry-info">
                    <span class="entry-type ${entry.trace_type.toLowerCase()}">${entry.trace_type}</span>
                    <span class="entry-operation">${entry.operation}</span>
                </div>
                <div class="entry-latency">${entry.latency.toFixed(2)}ms</div>
                <div class="entry-timestamp">${this.formatTimestamp(entry.timestamp)}</div>
            </div>
        `).join('');
    }

    initializeCharts() {
        // 처리율 차트
        const processingRateCtx = document.getElementById('processing-rate-chart').getContext('2d');
        this.charts.processingRate = new Chart(processingRateCtx, {
            type: 'line',
            data: {
                labels: [],
                datasets: [{
                    label: '처리율 (entries/sec)',
                    data: [],
                    borderColor: '#667eea',
                    backgroundColor: 'rgba(102, 126, 234, 0.1)',
                    tension: 0.4,
                    fill: true
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                scales: {
                    y: {
                        beginAtZero: true
                    }
                },
                plugins: {
                    legend: {
                        display: false
                    }
                },
                animation: {
                    duration: 0
                }
            }
        });

        // 레이턴시 차트
        const latencyCtx = document.getElementById('latency-chart').getContext('2d');
        this.charts.latency = new Chart(latencyCtx, {
            type: 'bar',
            data: {
                labels: ['평균', '최대', '최소'],
                datasets: [{
                    label: '레이턴시 (ms)',
                    data: [0, 0, 0],
                    backgroundColor: [
                        'rgba(102, 126, 234, 0.8)',
                        'rgba(239, 68, 68, 0.8)',
                        'rgba(16, 185, 129, 0.8)'
                    ],
                    borderColor: [
                        '#667eea',
                        '#ef4444',
                        '#10b981'
                    ],
                    borderWidth: 2
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                scales: {
                    y: {
                        beginAtZero: true
                    }
                },
                plugins: {
                    legend: {
                        display: false
                    }
                }
            }
        });

        // 트레이스 타입 차트
        const traceTypeCtx = document.getElementById('trace-type-chart').getContext('2d');
        this.charts.traceType = new Chart(traceTypeCtx, {
            type: 'doughnut',
            data: {
                labels: ['Block', 'UFS', 'Custom'],
                datasets: [{
                    data: [0, 0, 0],
                    backgroundColor: [
                        'rgba(102, 126, 234, 0.8)',
                        'rgba(16, 185, 129, 0.8)',
                        'rgba(245, 158, 11, 0.8)'
                    ],
                    borderColor: [
                        '#667eea',
                        '#10b981',
                        '#f59e0b'
                    ],
                    borderWidth: 2
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: {
                        position: 'bottom'
                    }
                }
            }
        });
    }

    initializeECharts() {
        // Block Scatter Chart
        this.eCharts.blockScatter = echarts.init(document.getElementById('block-scatter-chart'));
        this.eCharts.blockScatter.setOption({
            title: { text: 'Block I/O Scatter (시간 vs LBA/Sector)' },
            tooltip: {
                trigger: 'item',
                formatter: function(params) {
                    const relativeTime = params.value[0];
                    const lba = params.value[1];
                    const size = params.value[2];
                    const type = params.value[3];
                    return `상대시간: ${(relativeTime / 1000).toFixed(2)}s<br/>LBA/Sector: ${lba}<br/>Size: ${size}<br/>Type: ${type}`;
                }
            },
            xAxis: { 
                type: 'value', 
                name: 'Time (상대시간)',
                nameLocation: 'middle',
                nameGap: 30,
                axisLabel: {
                    formatter: function(value) {
                        return (value / 1000).toFixed(1) + 's';
                    }
                }
            },
            yAxis: { 
                type: 'value', 
                name: 'LBA/Sector',
                nameLocation: 'middle',
                nameGap: 50
            },
            series: [{
                type: 'scatter',
                data: [],
                symbolSize: function(data) {
                    return Math.min(Math.max(Math.log10(data[2] || 1) * 3, 3), 20);
                },
                itemStyle: {
                    color: function(params) {
                        const colors = { 'R': '#ff6b6b', 'W': '#4ecdc4', 'D': '#45b7d1', 'other': '#96ceb4' };
                        return colors[params.value[3]] || colors['other'];
                    }
                }
            }]
        });

        // UFS Scatter Chart
        this.eCharts.ufsScatter = echarts.init(document.getElementById('ufs-scatter-chart'));
        this.eCharts.ufsScatter.setOption({
            title: { text: 'UFS I/O Scatter (시간 vs LBA/Sector)' },
            tooltip: {
                trigger: 'item',
                formatter: function(params) {
                    const relativeTime = params.value[0];
                    const lba = params.value[1];
                    const size = params.value[2];
                    const opcode = params.value[3];
                    return `상대시간: ${(relativeTime / 1000).toFixed(2)}s<br/>LBA/Sector: ${lba}<br/>Size: ${size}<br/>Opcode: ${opcode}`;
                }
            },
            xAxis: { 
                type: 'value', 
                name: 'Time (상대시간)',
                nameLocation: 'middle',
                nameGap: 30,
                axisLabel: {
                    formatter: function(value) {
                        return (value / 1000).toFixed(1) + 's';
                    }
                }
            },
            yAxis: { 
                type: 'value', 
                name: 'LBA/Sector',
                nameLocation: 'middle',
                nameGap: 50
            },
            series: [{
                type: 'scatter',
                data: [],
                symbolSize: function(data) {
                    return Math.min(Math.max(Math.log10(data[2] || 1) * 3, 3), 20);
                },
                itemStyle: {
                    color: function(params) {
                        const colors = { 'READ': '#ff9f43', 'WRITE': '#6c5ce7', 'other': '#a29bfe' };
                        return colors[params.value[3]] || colors['other'];
                    }
                }
            }]
        });

        // UFSCUSTOM Scatter Chart
        this.eCharts.ufscustomScatter = echarts.init(document.getElementById('ufscustom-scatter-chart'));
        this.eCharts.ufscustomScatter.setOption({
            title: { text: 'UFSCUSTOM I/O Scatter (시간 vs LBA/Sector)' },
            tooltip: {
                trigger: 'item',
                formatter: function(params) {
                    const relativeTime = params.value[0];
                    const lba = params.value[1];
                    const size = params.value[2];
                    const opcode = params.value[3];
                    return `상대시간: ${(relativeTime / 1000).toFixed(2)}s<br/>LBA/Sector: ${lba}<br/>Size: ${size}<br/>Opcode: ${opcode}`;
                }
            },
            xAxis: { 
                type: 'value', 
                name: 'Time (상대시간)',
                nameLocation: 'middle',
                nameGap: 30,
                axisLabel: {
                    formatter: function(value) {
                        return (value / 1000).toFixed(1) + 's';
                    }
                }
            },
            yAxis: { 
                type: 'value', 
                name: 'LBA/Sector',
                nameLocation: 'middle',
                nameGap: 50
            },
            series: [{
                type: 'scatter',
                data: [],
                symbolSize: function(data) {
                    return Math.min(Math.max(Math.log10(data[2] || 1) * 3, 3), 20);
                },
                itemStyle: {
                    color: function(params) {
                        const colors = { 'READ': '#fd79a8', 'WRITE': '#fdcb6e', 'other': '#e17055' };
                        return colors[params.value[3]] || colors['other'];
                    }
                }
            }]
        });

        // Block Continuity Pie
        this.eCharts.blockContinuityPie = echarts.init(document.getElementById('block-continuity-pie'));
        this.eCharts.blockContinuityPie.setOption({
            title: { text: 'Block 주소 연속성' },
            tooltip: { trigger: 'item' },
            series: [{
                type: 'pie',
                radius: '50%',
                data: [
                    { value: 0, name: '연속적 Read' },
                    { value: 0, name: '비연속적 Read' },
                    { value: 0, name: '연속적 Write' },
                    { value: 0, name: '비연속적 Write' },
                    { value: 0, name: '연속적 Unmap' },
                    { value: 0, name: '비연속적 Unmap' }
                ],
                emphasis: {
                    itemStyle: {
                        shadowBlur: 10,
                        shadowOffsetX: 0,
                        shadowColor: 'rgba(0, 0, 0, 0.5)'
                    }
                }
            }]
        });

        // UFS Continuity Pie
        this.eCharts.ufsContinuityPie = echarts.init(document.getElementById('ufs-continuity-pie'));
        this.eCharts.ufsContinuityPie.setOption({
            title: { text: 'UFS 주소 연속성' },
            tooltip: { trigger: 'item' },
            series: [{
                type: 'pie',
                radius: '50%',
                data: [
                    { value: 0, name: '연속적 Read' },
                    { value: 0, name: '비연속적 Read' },
                    { value: 0, name: '연속적 Write' },
                    { value: 0, name: '비연속적 Write' },
                    { value: 0, name: '연속적 Unmap' },
                    { value: 0, name: '비연속적 Unmap' }
                ],
                emphasis: {
                    itemStyle: {
                        shadowBlur: 10,
                        shadowOffsetX: 0,
                        shadowColor: 'rgba(0, 0, 0, 0.5)'
                    }
                }
            }]
        });

        // UFSCUSTOM Continuity Pie
        this.eCharts.ufscustomContinuityPie = echarts.init(document.getElementById('ufscustom-continuity-pie'));
        this.eCharts.ufscustomContinuityPie.setOption({
            title: { text: 'UFSCUSTOM 주소 연속성' },
            tooltip: { trigger: 'item' },
            series: [{
                type: 'pie',
                radius: '50%',
                data: [
                    { value: 0, name: '연속적 Read' },
                    { value: 0, name: '비연속적 Read' },
                    { value: 0, name: '연속적 Write' },
                    { value: 0, name: '비연속적 Write' },
                    { value: 0, name: '연속적 Unmap' },
                    { value: 0, name: '비연속적 Unmap' }
                ],
                emphasis: {
                    itemStyle: {
                        shadowBlur: 10,
                        shadowOffsetX: 0,
                        shadowColor: 'rgba(0, 0, 0, 0.5)'
                    }
                }
            }]
        });
    }

    bindEvents() {
        // 페이지 가시성 변경 이벤트
        document.addEventListener('visibilitychange', () => {
            if (document.hidden) {
                console.log('페이지가 숨겨짐');
            } else {
                console.log('페이지가 표시됨');
                // 페이지가 다시 표시될 때 연결 상태 확인
                if (!this.isConnected) {
                    this.attemptReconnect();
                }
            }
        });

        // 윈도우 포커스 이벤트
        window.addEventListener('focus', () => {
            if (!this.isConnected) {
                this.attemptReconnect();
            }
        });
    }

    // 유틸리티 함수들
    formatNumber(num) {
        return num.toLocaleString();
    }

    formatTimestamp(timestamp) {
        const date = new Date(timestamp);
        return date.toLocaleTimeString();
    }

    getTrendIcon(direction) {
        switch (direction) {
            case 'increasing':
                return '⬆️';
            case 'decreasing':
                return '⬇️';
            case 'stable':
                return '➡️';
            default:
                return '❓';
        }
    }

    handleAlert(alert) {
        // 새로운 알림 처리
        console.log('새 알림:', alert);
        
        // 브라우저 알림 (권한이 있는 경우)
        if (Notification.permission === 'granted') {
            new Notification('로그 분석 알림', {
                body: alert.message,
                icon: '/static/icon.png'
            });
        }
    }

    handleAnomaly(anomaly) {
        // 이상 징후 처리
        console.log('이상 징후 감지:', anomaly);
    }

    // 수동 새로고침 기능
    refreshData() {
        console.log('수동 새로고침 요청');
        this.fetchDashboardData();
    }

    // 차트 데이터 내보내기
    exportChartData() {
        const data = {
            processingRate: this.charts.processingRate.data,
            latency: this.charts.latency.data,
            traceType: this.charts.traceType.data,
            timestamp: new Date().toISOString()
        };
        
        const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `dashboard-data-${Date.now()}.json`;
        a.click();
        URL.revokeObjectURL(url);
    }

    updateECharts(data) {
        // Block scatter 차트 업데이트
        if (data.block_traces && this.eCharts.blockScatter) {
            const blockData = data.block_traces.map(trace => [
                trace.timestamp || 0,
                trace.lba || 0,
                trace.size || 1,
                trace.io_type || 'other'
            ]);
            
            // 시간 범위 계산 (X축 최소값 설정)
            const timeValues = blockData.map(d => d[0]).filter(t => t > 0);
            const minTime = timeValues.length > 0 ? Math.min(...timeValues) : 0;
            const maxTime = timeValues.length > 0 ? Math.max(...timeValues) : 100;
            
            this.eCharts.blockScatter.setOption({
                xAxis: {
                    type: 'value',
                    name: 'Time (상대시간)',
                    nameLocation: 'middle',
                    nameGap: 30,
                    min: minTime,
                    max: maxTime,
                    axisLabel: {
                        formatter: function(value) {
                            return Math.round((value - minTime) / 1000) + 's';
                        }
                    }
                },
                yAxis: {
                    type: 'value',
                    name: 'LBA/Sector',
                    nameLocation: 'middle',
                    nameGap: 50
                },
                series: [{
                    data: blockData.slice(-1000) // 최근 1000개만 표시
                }]
            });
        }

        // UFS scatter 차트 업데이트
        if (data.ufs_traces && this.eCharts.ufsScatter) {
            const ufsData = data.ufs_traces.map(trace => [
                trace.timestamp || 0,
                trace.lba || 0,
                trace.size || 1,
                trace.opcode || 'other'
            ]);
            
            // 시간 범위 계산 (X축 최소값 설정)
            const timeValues = ufsData.map(d => d[0]).filter(t => t > 0);
            const minTime = timeValues.length > 0 ? Math.min(...timeValues) : 0;
            const maxTime = timeValues.length > 0 ? Math.max(...timeValues) : 100;
            
            this.eCharts.ufsScatter.setOption({
                xAxis: {
                    type: 'value',
                    name: 'Time (상대시간)',
                    nameLocation: 'middle',
                    nameGap: 30,
                    min: minTime,
                    max: maxTime,
                    axisLabel: {
                        formatter: function(value) {
                            return Math.round((value - minTime) / 1000) + 's';
                        }
                    }
                },
                yAxis: {
                    type: 'value',
                    name: 'LBA/Sector',
                    nameLocation: 'middle',
                    nameGap: 50
                },
                series: [{
                    data: ufsData.slice(-1000) // 최근 1000개만 표시
                }]
            });
        }

        // UFSCUSTOM scatter 차트 업데이트
        if (data.ufscustom_traces && this.eCharts.ufscustomScatter) {
            const ufscustomData = data.ufscustom_traces.map(trace => [
                trace.timestamp || 0,
                trace.lba || 0,
                trace.size || 1,
                trace.opcode || 'other'
            ]);
            
            // 시간 범위 계산 (X축 최소값 설정)
            const timeValues = ufscustomData.map(d => d[0]).filter(t => t > 0);
            const minTime = timeValues.length > 0 ? Math.min(...timeValues) : 0;
            const maxTime = timeValues.length > 0 ? Math.max(...timeValues) : 100;
            
            this.eCharts.ufscustomScatter.setOption({
                xAxis: {
                    type: 'value',
                    name: 'Time (상대시간)',
                    nameLocation: 'middle',
                    nameGap: 30,
                    min: minTime,
                    max: maxTime,
                    axisLabel: {
                        formatter: function(value) {
                            return Math.round((value - minTime) / 1000) + 's';
                        }
                    }
                },
                yAxis: {
                    type: 'value',
                    name: 'LBA/Sector',
                    nameLocation: 'middle',
                    nameGap: 50
                },
                series: [{
                    data: ufscustomData.slice(-1000) // 최근 1000개만 표시
                }]
            });
        }

        // 연속성 파이 차트 업데이트
        this.updateContinuityCharts(data);
    }

    updateContinuityCharts(data) {
        // Block 연속성 분석
        if (data.block_traces && this.eCharts.blockContinuityPie) {
            const continuity = this.analyzeContinuity(data.block_traces);
            this.eCharts.blockContinuityPie.setOption({
                series: [{
                    data: [
                        { value: continuity.continuous_read, name: '연속적 Read' },
                        { value: continuity.non_continuous_read, name: '비연속적 Read' },
                        { value: continuity.continuous_write, name: '연속적 Write' },
                        { value: continuity.non_continuous_write, name: '비연속적 Write' },
                        { value: continuity.continuous_unmap, name: '연속적 Unmap' },
                        { value: continuity.non_continuous_unmap, name: '비연속적 Unmap' }
                    ]
                }]
            });
        }

        // UFS 연속성 분석
        if (data.ufs_traces && this.eCharts.ufsContinuityPie) {
            const continuity = this.analyzeContinuity(data.ufs_traces);
            this.eCharts.ufsContinuityPie.setOption({
                series: [{
                    data: [
                        { value: continuity.continuous_read, name: '연속적 Read' },
                        { value: continuity.non_continuous_read, name: '비연속적 Read' },
                        { value: continuity.continuous_write, name: '연속적 Write' },
                        { value: continuity.non_continuous_write, name: '비연속적 Write' },
                        { value: continuity.continuous_unmap, name: '연속적 Unmap' },
                        { value: continuity.non_continuous_unmap, name: '비연속적 Unmap' }
                    ]
                }]
            });
        }

        // UFSCUSTOM 연속성 분석
        if (data.ufscustom_traces && this.eCharts.ufscustomContinuityPie) {
            const continuity = this.analyzeContinuity(data.ufscustom_traces);
            this.eCharts.ufscustomContinuityPie.setOption({
                series: [{
                    data: [
                        { value: continuity.continuous_read, name: '연속적 Read' },
                        { value: continuity.non_continuous_read, name: '비연속적 Read' },
                        { value: continuity.continuous_write, name: '연속적 Write' },
                        { value: continuity.non_continuous_write, name: '비연속적 Write' },
                        { value: continuity.continuous_unmap, name: '연속적 Unmap' },
                        { value: continuity.non_continuous_unmap, name: '비연속적 Unmap' }
                    ]
                }]
            });
        }
    }

    analyzeContinuity(traces) {
        const result = {
            continuous_read: 0,
            non_continuous_read: 0,
            continuous_write: 0,
            non_continuous_write: 0,
            continuous_unmap: 0,
            non_continuous_unmap: 0
        };

        if (traces.length < 2) return result;

        // 타입별로 그룹화
        const readTraces = traces.filter(t => this.isReadOperation(t));
        const writeTraces = traces.filter(t => this.isWriteOperation(t));
        const unmapTraces = traces.filter(t => this.isUnmapOperation(t));

        // 각 타입별 연속성 분석
        result.continuous_read = this.countContinuous(readTraces);
        result.non_continuous_read = readTraces.length - result.continuous_read;
        
        result.continuous_write = this.countContinuous(writeTraces);
        result.non_continuous_write = writeTraces.length - result.continuous_write;
        
        result.continuous_unmap = this.countContinuous(unmapTraces);
        result.non_continuous_unmap = unmapTraces.length - result.continuous_unmap;

        return result;
    }

    isReadOperation(trace) {
        if (trace.io_type) return trace.io_type.toLowerCase() === 'r';
        if (trace.opcode) return trace.opcode.toLowerCase().includes('read');
        return false;
    }

    isWriteOperation(trace) {
        if (trace.io_type) return trace.io_type.toLowerCase() === 'w';
        if (trace.opcode) return trace.opcode.toLowerCase().includes('write');
        return false;
    }

    isUnmapOperation(trace) {
        if (trace.io_type) return trace.io_type.toLowerCase() === 'd';
        if (trace.opcode) return trace.opcode.toLowerCase().includes('unmap');
        return false;
    }

    countContinuous(traces) {
        if (traces.length < 2) return 0;
        
        let continuous = 0;
        traces.sort((a, b) => (a.timestamp || 0) - (b.timestamp || 0));
        
        for (let i = 1; i < traces.length; i++) {
            const prev = traces[i - 1];
            const curr = traces[i];
            
            if (prev.lba && curr.lba && prev.size) {
                // 이전 요청의 끝 LBA와 현재 요청의 시작 LBA가 연속인지 확인
                if (prev.lba + prev.size === curr.lba) {
                    continuous++;
                }
            }
        }
        
        return continuous;
    }
}

// 페이지 로드 시 대시보드 초기화
document.addEventListener('DOMContentLoaded', () => {
    const dashboard = new RealTimeDashboard();
    
    // 전역 객체로 등록 (디버깅용)
    window.dashboard = dashboard;
    
    // 브라우저 알림 권한 요청
    if (Notification.permission === 'default') {
        Notification.requestPermission();
    }
    
    console.log('실시간 대시보드 초기화 완료');
});
