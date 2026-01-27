// User types
export interface User {
  id: number
  email: string
  coin_balance: number
  traffic_quota: number
  traffic_used: number
  referral_code: string | null
  status: string
  created_at: string
}

// Package types
export interface Package {
  id: number
  name: string
  traffic_amount: number
  price: number
  duration_days: number
  description: string | null
  is_active: boolean
}

// Order types
export interface Order {
  id: number
  order_no: string
  user_id: number
  package_id: number
  amount: number
  status: string
  created_at: string
  completed_at: string | null
  package?: Package
}

// Subscription types
export interface Subscription {
  token: string
  url: string
}

// Referral types
export interface ReferralStats {
  referral_count: number
  total_commission: number
  referral_link: string
}

// Node types
export interface Node {
  id: number
  name: string
  host: string
  port: number
  protocol: string
  status: string
}

// Traffic types
export interface TrafficInfo {
  traffic_quota: number
  traffic_used: number
  traffic_remaining: number
  percentage_used: number
}

// API Response types
export interface ApiResponse<T> {
  data?: T
  error?: {
    code: string
    message: string
    details?: any
  }
}

// Auth types
export interface LoginRequest {
  email: string
  password: string
}

export interface RegisterRequest {
  email: string
  password: string
  referral_code?: string
}

export interface AuthResponse {
  token: string
  user: User
}
