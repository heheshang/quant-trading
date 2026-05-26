import client from './client'
import type { PaginatedResponse } from '@/types'

export interface ReviewResponse {
  id: string
  strategy_id: string
  review_status: 'pending_review' | 'approved' | 'rejected'
  rejection_reason: string | null
  submitted_at: string | null
  reviewed_at: string | null
  reviewed_by: string | null
}

export interface SubmitReviewPayload {
  strategy_id: string
}

export interface ReviewDecisionPayload {
  strategy_id: string
  reason?: string
}

/** Submit a strategy for review */
export function submitForReview(payload: SubmitReviewPayload): Promise<ReviewResponse> {
  return client.post('/reviews/submit', payload)
}

/** Approve a strategy (admin) */
export function approveStrategy(payload: ReviewDecisionPayload): Promise<ReviewResponse> {
  return client.post('/reviews/approve', payload)
}

/** Reject a strategy (admin) */
export function rejectStrategy(payload: ReviewDecisionPayload): Promise<ReviewResponse> {
  return client.post('/reviews/reject', payload)
}

/** List all pending reviews (admin) */
export function listPendingReviews(): Promise<{ items: ReviewResponse[]; meta?: { page: number; size: number; total: number } }> {
  return client.get('/reviews/pending')
}

/** Get review for a specific strategy */
export function getStrategyReview(strategyId: string): Promise<ReviewResponse | null> {
  return client.get(`/reviews/${strategyId}`)
}
