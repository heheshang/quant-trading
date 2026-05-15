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

/**
 * 修正：后端 PaginatedResponse 直接返回 { items, total, page, size }，
 * 拦截器解包 body.data 后就是这个结构。
 * 不再 extends ApiResponse（ApiResponse 的 data 字段已由拦截器解开）。
 */
export interface PaginatedResponse<T> {
  items: T[]
  total: number
  page: number
  size: number
}
