export interface ApiResponse<T> {
  code: number
  data: T
  message: string
}

export interface PaginationMeta {
  page: number
  size: number
  total: number
}

export interface PaginatedResponse<T> extends ApiResponse<T> {
  meta: PaginationMeta
}
