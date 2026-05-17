import client from './client'
import type { User } from '@/types'

export function listUsers(): Promise<User[]> {
  return client.get('/users')
}

export function createUser(data: Partial<User>): Promise<User> {
  return client.post('/users', data)
}

export function updateUser(id: number, data: Partial<User>): Promise<User> {
  return client.put(`/users/${id}`, data)
}

export function deleteUser(id: number): Promise<void> {
  return client.delete(`/users/${id}`)
}
