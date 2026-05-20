import client from './client'
import type {
  ApiKey,
  CreateApiKeyRequest,
  UpdateApiKeyRequest,
  ApiKeyTestResult,
  ApiKeyListResponse,
} from '@/types/apiKey'

/** Get current user's API keys: GET /api/v1/api-keys */
export function getApiKeys(): Promise<ApiKey[]> {
  return client.get('/api-keys')
}

/** Get paginated API keys: GET /api/v1/api-keys?page=1&size=20 */
export function getApiKeysPaginated(params?: { page?: number; size?: number }): Promise<ApiKeyListResponse> {
  return client.get('/api-keys', { params })
}

/** Create API key: POST /api/v1/api-keys */
export function createApiKey(data: CreateApiKeyRequest): Promise<ApiKey> {
  return client.post('/api-keys', data)
}

/** Update API key: PUT /api/v1/api-keys/:id */
export function updateApiKey(id: string, data: UpdateApiKeyRequest): Promise<ApiKey> {
  return client.put(`/api-keys/${id}`, data)
}

/** Delete API key: DELETE /api/v1/api-keys/:id */
export function deleteApiKey(id: string): Promise<void> {
  return client.delete(`/api-keys/${id}`)
}

/** Test API key connectivity: POST /api/v1/api-keys/:id/test */
export function testApiKey(id: string): Promise<ApiKeyTestResult> {
  return client.post(`/api-keys/${id}/test`)
}

/** Get admin API keys list: GET /api/v1/admin/api-keys */
export function getAdminApiKeys(params?: { page?: number; size?: number; user_id?: string }): Promise<ApiKeyListResponse> {
  return client.get('/admin/api-keys', { params })
}
