/** API Key permission types */
export type ApiKeyPermission = 'read' | 'trade' | 'withdraw'

/** Supported exchanges */
export type Exchange = 'binance' | 'okx' | 'huobi' | 'bybit' | 'gateio' | 'kucoin'

/** API Key entity from backend */
export interface ApiKey {
  id: string
  user_id: string
  exchange: Exchange
  api_key: string          // Masked, e.g. "***ABCD"
  permissions: ApiKeyPermission[]
  is_active: boolean
  last_used_at: string | null
  created_at: string
}

/** Create API Key request */
export interface CreateApiKeyRequest {
  exchange: Exchange
  api_key: string
  secret_key: string
  permissions: ApiKeyPermission[]
  is_active?: boolean
}

/** Update API Key request */
export interface UpdateApiKeyRequest {
  exchange?: Exchange
  api_key?: string
  secret_key?: string
  permissions?: ApiKeyPermission[]
  is_active?: boolean
}

/** API Key test result */
export interface ApiKeyTestResult {
  success: boolean
  message: string
  exchange?: string
  server_time?: string
  permissions?: ApiKeyPermission[]
}

/** API Key list response */
export interface ApiKeyListResponse {
  items: ApiKey[]
  total: number
  page: number
  size: number
}

/** Exchange display info */
export const EXCHANGE_INFO: Record<Exchange, { label: string; icon: string }> = {
  binance: { label: 'Binance', icon: 'BN' },
  okx: { label: 'OKX', icon: 'OK' },
  huobi: { label: 'Huobi', icon: 'HT' },
  bybit: { label: 'Bybit', icon: 'BY' },
  gateio: { label: 'Gate.io', icon: 'GT' },
  kucoin: { label: 'KuCoin', icon: 'KC' },
}

/** Permission display info */
export const PERMISSION_INFO: Record<ApiKeyPermission, { label: string; description: string }> = {
  read: { label: '读取', description: '查看账户余额、订单、持仓等信息' },
  trade: { label: '交易', description: '下单、撤单等交易操作' },
  withdraw: { label: '提现', description: '提币权限（请谨慎开启）' },
}

/** Format permission array to display string */
export function formatPermissions(permissions: ApiKeyPermission[]): string {
  return permissions.map(p => PERMISSION_INFO[p]?.label || p).join(', ')
}

/** Get masked API key display */
export function maskApiKey(key: string): string {
  if (key.length <= 8) return `***${key}`
  return `***${key.slice(-8)}`
}
