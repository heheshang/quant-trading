// useRiskDashboard.ts — Risk Dashboard Composable
// API base: /api/v1/risk/
import { ref, computed } from 'vue'
import * as riskApi from '@/api/risk'
import type {
  RiskRules,
  RiskLog,
  EmergencyCloseResponse,
  ConnectionStatus,
} from '@/types/risk'

export function useRiskDashboard() {
  // ─── State ────────────────────────────────────────────────────
  const rules = ref<RiskRules | null>(null)
  const logs = ref<RiskLog[]>([])
  const logsTotal = ref(0)
  const connectionStatus = ref<ConnectionStatus | null>(null)

  const rulesLoading = ref(false)
  const logsLoading = ref(false)
  const connectionLoading = ref(false)
  const emergencyLoading = ref(false)

  const rulesError = ref<string | null>(null)
  const logsError = ref<string | null>(null)
  const connectionError = ref<string | null>(null)
  const emergencyError = ref<string | null>(null)

  // ─── Computed: connection status ──────────────────────────────
  const isDisconnected = computed(
    () => connectionStatus.value?.exchange_connected === false,
  )

  const disconnectLevel = computed(() => {
    if (!connectionStatus.value) return 'unknown'
    const elapsed = connectionStatus.value.disconnect_elapsed_secs
    if (elapsed === 0) return 'connected'
    if (elapsed < 30) return 'warning'
    return 'critical'
  })

  const disconnectLabel = computed(() => {
    const s = connectionStatus.value
    if (!s) return '未知'
    if (s.exchange_connected === false) return '已断线'
    const elapsed = s.disconnect_elapsed_secs
    if (elapsed === 0) return '已连接'
    if (elapsed < 60) return `断线 ${elapsed}s`
    return `断线 ${Math.floor(elapsed / 60)}m ${elapsed % 60}s`
  })

  // ─── Fetch: risk rules ────────────────────────────────────────
  async function fetchRules() {
    rulesLoading.value = true
    rulesError.value = null
    try {
      const res = await riskApi.getRiskRules()
      rules.value = res.data ?? null
    } catch (err: unknown) {
      rulesError.value = err instanceof Error ? err.message : String(err) || '获取风控规则失败'
    } finally {
      rulesLoading.value = false
    }
  }

  // ─── Fetch: risk logs (paginated) ─────────────────────────────
  async function fetchLogs(page = 1, pageSize = 20) {
    logsLoading.value = true
    logsError.value = null
    try {
      const res = await riskApi.getRiskLogs({ page, page_size: pageSize })
      logs.value = res.data?.data ?? []
      logsTotal.value = res.data?.total ?? 0
    } catch (err: unknown) {
      logsError.value = err instanceof Error ? err.message : String(err) || '获取风控日志失败'
    } finally {
      logsLoading.value = false
    }
  }

  // ─── Fetch: connection status ────────────────────────────────
  async function fetchConnectionStatus() {
    connectionLoading.value = true
    connectionError.value = null
    try {
      const res = await riskApi.getConnectionStatus()
      connectionStatus.value = res.data ?? null
    } catch (err: unknown) {
      connectionError.value = err instanceof Error ? err.message : String(err) || '获取连接状态失败'
    } finally {
      connectionLoading.value = false
    }
  }

  // ─── Action: emergency close ──────────────────────────────────
  async function triggerEmergencyClose(): Promise<EmergencyCloseResponse | null> {
    emergencyLoading.value = true
    emergencyError.value = null
    try {
      const res = await riskApi.emergencyClose()
      return res.data ?? null
    } catch (err: unknown) {
      emergencyError.value = err instanceof Error ? err.message : String(err) || '紧急全平失败'
      return null
    } finally {
      emergencyLoading.value = false
    }
  }

  // ─── Refresh all ──────────────────────────────────────────────
  async function refresh() {
    await Promise.all([fetchRules(), fetchLogs(), fetchConnectionStatus()])
  }

  return {
    // state
    rules,
    logs,
    logsTotal,
    connectionStatus,
    rulesLoading,
    logsLoading,
    connectionLoading,
    emergencyLoading,
    rulesError,
    logsError,
    connectionError,
    emergencyError,
    // computed
    isDisconnected,
    disconnectLevel,
    disconnectLabel,
    // actions
    fetchRules,
    fetchLogs,
    fetchConnectionStatus,
    triggerEmergencyClose,
    refresh,
  }
}
