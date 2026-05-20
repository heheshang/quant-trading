// Risk Management API — P0-F2 / P0-F3
// Base path: /api/v1/risk

import type { ApiResponse } from '@/types/exchange'
import type {
  RiskRules,
  RiskLogsResponse,
  EmergencyCloseResponse,
  PauseResponse,
  ConnectionStatus,
} from '@/types/risk'
import client from './client'

// ─── P0-F2: 资金风控规则 ────────────────────────────────────────

/** GET /api/v1/risk/rules — 获取当前用户风控规则 */
export function getRiskRules(): Promise<ApiResponse<RiskRules>> {
  return client.get('/risk/rules')
}

/** PUT /api/v1/risk/rules — 更新风控规则 */
export function updateRiskRules(
  rules: Partial<RiskRules>
): Promise<ApiResponse<RiskRules>> {
  return client.put('/risk/rules', rules)
}

// ─── P0-F2: 风控日志 ────────────────────────────────────────────

/** GET /api/v1/risk/logs — 分页查询风控日志 */
export function getRiskLogs(params?: {
  page?: number
  page_size?: number
}): Promise<ApiResponse<RiskLogsResponse>> {
  return client.get('/risk/logs', { params })
}

// ─── P0-F3: 应急操作 ────────────────────────────────────────────

/** POST /api/v1/risk/emergency-close — 紧急全平 */
export function emergencyClose(): Promise<ApiResponse<EmergencyCloseResponse>> {
  return client.post('/risk/emergency-close', {})
}

/** POST /api/v1/risk/pause — 暂停交易 */
export function pauseTrading(): Promise<ApiResponse<PauseResponse>> {
  return client.post('/risk/pause', {})
}

/** POST /api/v1/risk/resume — 恢复交易 */
export function resumeTrading(): Promise<ApiResponse<PauseResponse>> {
  return client.post('/risk/resume', {})
}

/** POST /api/v1/risk/check — 手动触发风控检查 */
export function manualRiskCheck(): Promise<ApiResponse<unknown>> {
  return client.post('/risk/check', {})
}

// ─── P0-F3: 连接状态 ────────────────────────────────────────────

/** GET /api/v1/risk/connection-status — WebSocket/Binance 连接真实状态 */
export function getConnectionStatus(): Promise<ApiResponse<ConnectionStatus>> {
  return client.get('/risk/connection-status')
}
