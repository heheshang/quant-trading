export interface LoginRequest {
  username: string
  password: string
}

export interface RegisterRequest {
  username: string
  password: string
  email: string
}

export interface AuthResponse {
  access_token: string
  refresh_token: string
  user: UserInfo
}

export interface UserInfo {
  id: number
  username: string
  email: string
  role: 'admin' | 'user'
  created_at: string
}

export interface RefreshRequest {
  refresh_token: string
}
