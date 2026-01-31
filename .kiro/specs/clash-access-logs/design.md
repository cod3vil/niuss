# Design Document: Clash Access Logs

## Overview

This design implements a comprehensive access logging system for Clash VPN subscription URLs. The system will track every access attempt to subscription configurations, store detailed access information in a PostgreSQL database, and provide an administrative interface for viewing and analyzing access patterns.

The implementation follows the existing codebase patterns:
- Rust backend using Axum framework with SQLx for database operations
- PostgreSQL database with migration-based schema management
- Vue.js admin panel with TypeScript
- Redis caching for performance optimization
- RESTful API design with JWT authentication

## Architecture

### System Components

```
┌─────────────────┐
│  Clash Client   │
│                 │
└────────┬────────┘
         │ GET /sub/:token
         ▼
┌─────────────────────────────────────────┐
│         Axum HTTP Handler               │
│  (get_subscription_config_handler)      │
│                                         │
│  1. Extract request metadata            │
│  2. Log access attempt                  │
│  3. Process subscription request        │
└────────┬────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│      Access Logging Service             │
│                                         │
│  - Extract IP address                   │
│  - Extract User-Agent                   │
│  - Determine response status            │
│  - Write to database (async)            │
└────────┬────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│      PostgreSQL Database                │
│                                         │
│  clash_access_logs table                │
│  - user_id, token, timestamp            │
│  - ip_address, user_agent               │
│  - response_status                      │
└─────────────────────────────────────────┘

┌─────────────────┐
│  Admin Panel    │
│  (Vue.js)       │
└────────┬────────┘
         │ GET /api/admin/access-logs
         ▼
┌─────────────────────────────────────────┐
│      Admin API Handler                  │
│                                         │
│  - Verify admin authentication          │
│  - Parse filter parameters              │
│  - Query database with filters          │
│  - Return paginated results             │
└─────────────────────────────────────────┘
```

### Data Flow

1. **Access Logging Flow**:
   - Client requests `/sub/:token`
   - Handler extracts request metadata (IP, User-Agent)
   - Handler processes subscription request
   - Handler logs access attempt asynchronously (non-blocking)
   - Response returned to client

2. **Admin Query Flow**:
   - Admin requests access logs via API
   - API validates admin JWT token
   - API applies filters (user_id, date range, status)
   - Database returns paginated results with user email joined
   - API returns formatted response

## Components and Interfaces

### Database Schema

#### clash_access_logs Table

```sql
CREATE TABLE clash_access_logs (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subscription_token VARCHAR(64) NOT NULL,
    access_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address VARCHAR(45) NOT NULL,  -- IPv6 max length
    user_agent TEXT,
    response_status VARCHAR(20) NOT NULL CHECK (response_status IN ('success', 'failed', 'quota_exceeded', 'expired', 'disabled')),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_clash_access_logs_user_id ON clash_access_logs(user_id);
CREATE INDEX idx_clash_access_logs_access_timestamp ON clash_access_logs(access_timestamp);
CREATE INDEX idx_clash_access_logs_response_status ON clash_access_logs(response_status);
CREATE INDEX idx_clash_access_logs_subscription_token ON clash_access_logs(subscription_token);
```

**Design Decisions**:
- `ip_address` as VARCHAR(45) to support both IPv4 and IPv6
- `user_agent` as TEXT to accommodate long user agent strings
- `response_status` enum includes all possible subscription access outcomes
- Indexes on `user_id`, `access_timestamp`, and `response_status` for efficient filtering
- Foreign key to `users` table with CASCADE delete to maintain referential integrity

### Backend Models

#### ClashAccessLog Model (models.rs)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// ClashAccessLog model representing a subscription access attempt
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ClashAccessLog {
    pub id: i64,
    pub user_id: i64,
    pub subscription_token: String,
    pub access_timestamp: DateTime<Utc>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub response_status: String,
    pub created_at: DateTime<Utc>,
}

/// Request body for querying access logs (admin)
#[derive(Debug, Deserialize)]
pub struct AccessLogQueryRequest {
    pub user_id: Option<i64>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// Response for access log query with user information
#[derive(Debug, Serialize)]
pub struct AccessLogResponse {
    pub id: i64,
    pub user_id: i64,
    pub user_email: String,
    pub subscription_token: String,
    pub access_timestamp: DateTime<Utc>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub response_status: String,
}

/// Paginated response for access logs
#[derive(Debug, Serialize)]
pub struct AccessLogListResponse {
    pub logs: Vec<AccessLogResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}
```

### Backend Database Functions (db.rs)

```rust
/// Create a new access log entry
pub async fn create_access_log(
    pool: &PgPool,
    user_id: i64,
    subscription_token: &str,
    ip_address: &str,
    user_agent: Option<&str>,
    response_status: &str,
) -> Result<ClashAccessLog, sqlx::Error> {
    sqlx::query_as::<_, ClashAccessLog>(
        r#"
        INSERT INTO clash_access_logs 
        (user_id, subscription_token, access_timestamp, ip_address, user_agent, response_status)
        VALUES ($1, $2, NOW(), $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(subscription_token)
    .bind(ip_address)
    .bind(user_agent)
    .bind(response_status)
    .fetch_one(pool)
    .await
}

/// Query access logs with filters and pagination
pub async fn query_access_logs(
    pool: &PgPool,
    user_id: Option<i64>,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    status: Option<&str>,
    page: i64,
    page_size: i64,
) -> Result<(Vec<AccessLogResponse>, i64), sqlx::Error> {
    let offset = (page - 1) * page_size;
    
    // Build dynamic query based on filters
    let mut query = String::from(
        r#"
        SELECT 
            cal.id,
            cal.user_id,
            u.email as user_email,
            cal.subscription_token,
            cal.access_timestamp,
            cal.ip_address,
            cal.user_agent,
            cal.response_status
        FROM clash_access_logs cal
        INNER JOIN users u ON cal.user_id = u.id
        WHERE 1=1
        "#
    );
    
    let mut count_query = String::from(
        r#"
        SELECT COUNT(*) as count
        FROM clash_access_logs cal
        WHERE 1=1
        "#
    );
    
    // Add filters dynamically
    if user_id.is_some() {
        query.push_str(" AND cal.user_id = $1");
        count_query.push_str(" AND cal.user_id = $1");
    }
    
    if start_date.is_some() {
        query.push_str(" AND cal.access_timestamp >= $2");
        count_query.push_str(" AND cal.access_timestamp >= $2");
    }
    
    if end_date.is_some() {
        query.push_str(" AND cal.access_timestamp <= $3");
        count_query.push_str(" AND cal.access_timestamp <= $3");
    }
    
    if status.is_some() {
        query.push_str(" AND cal.response_status = $4");
        count_query.push_str(" AND cal.response_status = $4");
    }
    
    query.push_str(" ORDER BY cal.access_timestamp DESC LIMIT $5 OFFSET $6");
    
    // Execute queries with proper parameter binding
    // (Actual implementation will use sqlx::query_as with bind() calls)
    
    // Return (logs, total_count)
}
```

### Backend API Handlers (handlers.rs)

#### Modified Subscription Handler

```rust
/// GET /sub/:token - Get Clash subscription configuration (with access logging)
async fn get_subscription_config_handler(
    State(state): State<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    // Extract IP address from headers
    let ip_address = extract_client_ip(&headers)
        .unwrap_or_else(|| "unknown".to_string());
    
    // Extract User-Agent
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    
    // Get subscription from database
    let subscription = match db::get_subscription_by_token(&state.db_pool, &token).await {
        Ok(Some(sub)) => sub,
        Ok(None) => {
            // Log failed access attempt (no user_id available)
            return Err(ApiError::NotFound("Subscription not found".to_string()));
        }
        Err(e) => return Err(e.into()),
    };
    
    let user_id = subscription.user_id;
    
    // Get user
    let user = match db::get_user_by_id(&state.db_pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            log_access_async(&state, user_id, &token, &ip_address, user_agent.as_deref(), "failed").await;
            return Err(ApiError::NotFound("User not found".to_string()));
        }
        Err(e) => {
            log_access_async(&state, user_id, &token, &ip_address, user_agent.as_deref(), "failed").await;
            return Err(e.into());
        }
    };
    
    // Check user status
    if user.status == "disabled" {
        log_access_async(&state, user_id, &token, &ip_address, user_agent.as_deref(), "disabled").await;
        return Err(ApiError::Unauthorized("Account is disabled".to_string()));
    }
    
    // Check traffic quota
    let has_traffic = traffic::check_traffic_quota(&state.db_pool, user.id)
        .await
        .unwrap_or(false);
    
    if !has_traffic {
        log_access_async(&state, user_id, &token, &ip_address, user_agent.as_deref(), "quota_exceeded").await;
        let empty_config = "proxies: []\nproxy-groups: []\nrules: []\n";
        return Ok((
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
            empty_config.to_string(),
        ));
    }
    
    // ... rest of existing logic ...
    
    // Log successful access
    log_access_async(&state, user_id, &token, &ip_address, user_agent.as_deref(), "success").await;
    
    // Return configuration
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
        clash_config,
    ))
}

/// Helper function to extract client IP from headers
fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    // Check X-Forwarded-For header first (for proxied requests)
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            // Take the first IP in the chain
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return Some(first_ip.trim().to_string());
            }
        }
    }
    
    // Check X-Real-IP header
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return Some(ip_str.to_string());
        }
    }
    
    None
}

/// Async helper to log access without blocking the response
async fn log_access_async(
    state: &AppState,
    user_id: i64,
    token: &str,
    ip_address: &str,
    user_agent: Option<&str>,
    status: &str,
) {
    let pool = state.db_pool.clone();
    let token = token.to_string();
    let ip = ip_address.to_string();
    let ua = user_agent.map(|s| s.to_string());
    let status = status.to_string();
    
    // Spawn async task to avoid blocking
    tokio::spawn(async move {
        if let Err(e) = db::create_access_log(
            &pool,
            user_id,
            &token,
            &ip,
            ua.as_deref(),
            &status,
        ).await {
            tracing::error!("Failed to log access: {:?}", e);
        }
    });
}
```

#### Admin Access Log Query Handler

```rust
/// GET /api/admin/access-logs - Query access logs (admin only)
async fn admin_query_access_logs_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<AccessLogQueryRequest>,
) -> Result<Json<AccessLogListResponse>, ApiError> {
    // Verify admin authentication
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing authorization header".to_string()))?;
    
    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid token".to_string()))?;
    
    // Verify admin status
    let user = db::get_user_by_id(&state.db_pool, claims.sub)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;
    
    if !user.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }
    
    // Set defaults for pagination
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(50).clamp(1, 100);
    
    // Query access logs
    let (logs, total) = db::query_access_logs(
        &state.db_pool,
        params.user_id,
        params.start_date,
        params.end_date,
        params.status.as_deref(),
        page,
        page_size,
    ).await?;
    
    let total_pages = (total + page_size - 1) / page_size;
    
    Ok(Json(AccessLogListResponse {
        logs,
        total,
        page,
        page_size,
        total_pages,
    }))
}
```

### Frontend Components

#### Access Logs View (admin/src/views/AccessLogs.vue)

```vue
<template>
  <div class="access-logs">
    <h1>Access Logs</h1>
    
    <!-- Filters -->
    <div class="filters">
      <div class="filter-group">
        <label>User Search:</label>
        <input 
          v-model="filters.userSearch" 
          type="text" 
          placeholder="Email or User ID"
          @input="debouncedSearch"
        />
      </div>
      
      <div class="filter-group">
        <label>Date Range:</label>
        <input v-model="filters.startDate" type="datetime-local" />
        <span>to</span>
        <input v-model="filters.endDate" type="datetime-local" />
      </div>
      
      <div class="filter-group">
        <label>Status:</label>
        <select v-model="filters.status">
          <option value="">All</option>
          <option value="success">Success</option>
          <option value="failed">Failed</option>
          <option value="quota_exceeded">Quota Exceeded</option>
          <option value="expired">Expired</option>
          <option value="disabled">Disabled</option>
        </select>
      </div>
      
      <button @click="applyFilters">Apply Filters</button>
      <button @click="clearFilters">Clear Filters</button>
    </div>
    
    <!-- Logs Table -->
    <div class="logs-table">
      <table>
        <thead>
          <tr>
            <th>Time</th>
            <th>User</th>
            <th>IP Address</th>
            <th>User Agent</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="log in logs" :key="log.id">
            <td>{{ formatTimestamp(log.access_timestamp) }}</td>
            <td>{{ log.user_email }}</td>
            <td>{{ log.ip_address }}</td>
            <td class="user-agent">{{ log.user_agent || 'N/A' }}</td>
            <td>
              <span :class="['status-badge', statusClass(log.response_status)]">
                {{ log.response_status }}
              </span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    
    <!-- Pagination -->
    <div class="pagination">
      <button @click="prevPage" :disabled="currentPage === 1">Previous</button>
      <span>Page {{ currentPage }} of {{ totalPages }}</span>
      <button @click="nextPage" :disabled="currentPage === totalPages">Next</button>
      <span class="total-count">Total: {{ totalLogs }} logs</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useAccessLogsStore } from '@/stores/accessLogs';
import { debounce } from 'lodash-es';

const accessLogsStore = useAccessLogsStore();

const filters = ref({
  userSearch: '',
  startDate: '',
  endDate: '',
  status: '',
});

const logs = ref([]);
const currentPage = ref(1);
const totalPages = ref(1);
const totalLogs = ref(0);

const loadLogs = async () => {
  const result = await accessLogsStore.fetchAccessLogs({
    ...filters.value,
    page: currentPage.value,
  });
  
  logs.value = result.logs;
  totalPages.value = result.total_pages;
  totalLogs.value = result.total;
};

const applyFilters = () => {
  currentPage.value = 1;
  loadLogs();
};

const clearFilters = () => {
  filters.value = {
    userSearch: '',
    startDate: '',
    endDate: '',
    status: '',
  };
  applyFilters();
};

const debouncedSearch = debounce(() => {
  applyFilters();
}, 500);

const prevPage = () => {
  if (currentPage.value > 1) {
    currentPage.value--;
    loadLogs();
  }
};

const nextPage = () => {
  if (currentPage.value < totalPages.value) {
    currentPage.value++;
    loadLogs();
  }
};

const formatTimestamp = (timestamp: string) => {
  return new Date(timestamp).toLocaleString();
};

const statusClass = (status: string) => {
  const classes = {
    success: 'status-success',
    failed: 'status-failed',
    quota_exceeded: 'status-warning',
    expired: 'status-warning',
    disabled: 'status-failed',
  };
  return classes[status] || 'status-default';
};

onMounted(() => {
  loadLogs();
});
</script>

<style scoped>
.access-logs {
  padding: 20px;
}

.filters {
  display: flex;
  gap: 15px;
  margin-bottom: 20px;
  flex-wrap: wrap;
}

.filter-group {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.logs-table {
  overflow-x: auto;
}

table {
  width: 100%;
  border-collapse: collapse;
}

th, td {
  padding: 12px;
  text-align: left;
  border-bottom: 1px solid #ddd;
}

.user-agent {
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-badge {
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: bold;
}

.status-success {
  background-color: #d4edda;
  color: #155724;
}

.status-failed {
  background-color: #f8d7da;
  color: #721c24;
}

.status-warning {
  background-color: #fff3cd;
  color: #856404;
}

.pagination {
  display: flex;
  gap: 10px;
  align-items: center;
  margin-top: 20px;
}

.total-count {
  margin-left: auto;
  color: #666;
}
</style>
```

#### Access Logs Store (admin/src/stores/accessLogs.ts)

```typescript
import { defineStore } from 'pinia';
import api from '@/api';

export interface AccessLog {
  id: number;
  user_id: number;
  user_email: string;
  subscription_token: string;
  access_timestamp: string;
  ip_address: string;
  user_agent: string | null;
  response_status: string;
}

export interface AccessLogFilters {
  userSearch?: string;
  startDate?: string;
  endDate?: string;
  status?: string;
  page?: number;
  pageSize?: number;
}

export interface AccessLogListResponse {
  logs: AccessLog[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export const useAccessLogsStore = defineStore('accessLogs', {
  state: () => ({
    logs: [] as AccessLog[],
    loading: false,
    error: null as string | null,
  }),

  actions: {
    async fetchAccessLogs(filters: AccessLogFilters): Promise<AccessLogListResponse> {
      this.loading = true;
      this.error = null;

      try {
        const params = new URLSearchParams();
        
        if (filters.userSearch) {
          // Try to parse as user ID, otherwise search by email
          const userId = parseInt(filters.userSearch);
          if (!isNaN(userId)) {
            params.append('user_id', userId.toString());
          }
          // Note: Email search would require backend support
        }
        
        if (filters.startDate) {
          params.append('start_date', new Date(filters.startDate).toISOString());
        }
        
        if (filters.endDate) {
          params.append('end_date', new Date(filters.endDate).toISOString());
        }
        
        if (filters.status) {
          params.append('status', filters.status);
        }
        
        params.append('page', (filters.page || 1).toString());
        params.append('page_size', (filters.pageSize || 50).toString());

        const response = await api.get(`/api/admin/access-logs?${params.toString()}`);
        
        this.logs = response.data.logs;
        return response.data;
      } catch (error: any) {
        this.error = error.response?.data?.error?.message || 'Failed to fetch access logs';
        throw error;
      } finally {
        this.loading = false;
      }
    },
  },
});
```

## Data Models

### Database Table: clash_access_logs

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | BIGSERIAL | PRIMARY KEY | Unique identifier |
| user_id | BIGINT | NOT NULL, FK to users(id) | User who accessed |
| subscription_token | VARCHAR(64) | NOT NULL | Token used for access |
| access_timestamp | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | When access occurred |
| ip_address | VARCHAR(45) | NOT NULL | Client IP address |
| user_agent | TEXT | NULL | Client user agent string |
| response_status | VARCHAR(20) | NOT NULL, CHECK constraint | Access result status |
| created_at | TIMESTAMPTZ | DEFAULT NOW() | Record creation time |

**Indexes**:
- `idx_clash_access_logs_user_id` on `user_id`
- `idx_clash_access_logs_access_timestamp` on `access_timestamp`
- `idx_clash_access_logs_response_status` on `response_status`
- `idx_clash_access_logs_subscription_token` on `subscription_token`

### Response Status Values

- `success`: Subscription configuration returned successfully
- `failed`: General failure (token not found, user not found)
- `quota_exceeded`: User exceeded traffic quota
- `expired`: User package expired
- `disabled`: User account disabled

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*


### Property 1: Automatic Timestamp Recording

*For any* access log entry created, the access_timestamp field should be automatically populated with a timestamp close to the current time (within a reasonable delta, e.g., 5 seconds).

**Validates: Requirements 1.2**

### Property 2: Access Logging on Subscription Request

*For any* subscription access attempt (valid or invalid token), a log entry should be created in the database with the user_id, token, and access metadata.

**Validates: Requirements 2.1**

### Property 3: Request Metadata Extraction

*For any* HTTP request to the subscription endpoint, the log entry should contain the client IP address extracted from headers (X-Forwarded-For, X-Real-IP, or connection) and the User-Agent header value if present.

**Validates: Requirements 2.2, 2.3**

### Property 4: Response Status Recording

*For any* subscription access attempt, the response_status field should accurately reflect the outcome: "success" for successful requests, "failed" for invalid tokens, "quota_exceeded" for traffic limit violations, "expired" for expired packages, and "disabled" for disabled accounts.

**Validates: Requirements 2.4, 2.5**

### Property 5: Comprehensive Filter Application

*For any* combination of filters (user_id, date range, status), all returned access logs should match ALL specified filter criteria simultaneously (AND logic).

**Validates: Requirements 3.2, 3.3, 3.4, 5.5**

### Property 6: Descending Timestamp Ordering

*For any* query result from the access log API, each log entry should have an access_timestamp greater than or equal to the next entry's timestamp (newest first ordering).

**Validates: Requirements 3.5**

### Property 7: User Email Join Consistency

*For any* access log entry returned by the API, the user_email field should match the email of the user with the corresponding user_id from the users table.

**Validates: Requirements 3.6**

### Property 8: Pagination Total Count Accuracy

*For any* query with pagination parameters, the total count returned should equal the actual number of records matching the filter criteria in the database.

**Validates: Requirements 3.8**

### Property 9: Timestamp Formatting Consistency

*For any* timestamp value, the formatting function should produce a human-readable string that includes date, time, and timezone information in a consistent format.

**Validates: Requirements 4.3**

## Error Handling

### Logging Failures

**Strategy**: Logging operations must never block or fail the main subscription request flow.

**Implementation**:
- Use `tokio::spawn` to execute logging asynchronously
- Log errors to application logs if database write fails
- Continue serving subscription configuration even if logging fails
- Monitor logging failure rates through application metrics

### Database Unavailability

**Strategy**: Gracefully degrade when database is unavailable.

**Implementation**:
- Wrap database calls in error handling
- Return subscription configuration even if logging fails
- Log warning messages for monitoring
- Use connection pooling with timeout settings

### Invalid Input Handling

**Strategy**: Validate and sanitize all input data.

**Implementation**:
- Validate IP address format (IPv4/IPv6)
- Truncate excessively long user agent strings
- Validate response_status enum values
- Use parameterized queries to prevent SQL injection

### API Error Responses

**Strategy**: Return appropriate HTTP status codes and error messages.

**Implementation**:
- 401 Unauthorized: Missing or invalid admin token
- 403 Forbidden: Non-admin user attempting access
- 400 Bad Request: Invalid filter parameters
- 500 Internal Server Error: Database or server errors

## Testing Strategy

### Dual Testing Approach

This feature requires both unit tests and property-based tests for comprehensive coverage:

**Unit Tests** focus on:
- Specific examples of access logging scenarios
- Edge cases (empty user agent, IPv6 addresses, special characters)
- Error conditions (database failures, invalid tokens)
- Integration points (handler to database, API to frontend)

**Property-Based Tests** focus on:
- Universal properties that hold for all inputs
- Comprehensive input coverage through randomization
- Minimum 100 iterations per property test

### Property-Based Testing Configuration

**Library**: Use `proptest` crate for Rust backend testing

**Test Configuration**:
- Minimum 100 iterations per property test
- Each test tagged with: **Feature: clash-access-logs, Property {number}: {property_text}**
- Each correctness property implemented by a SINGLE property-based test

**Example Property Test Structure**:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    // Feature: clash-access-logs, Property 1: Automatic Timestamp Recording
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn test_automatic_timestamp_recording(
            user_id in 1..=1000000i64,
            token in "[a-zA-Z0-9]{32,64}",
            ip in "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",
        ) {
            // Create access log
            let before = Utc::now();
            let log = create_access_log(user_id, &token, &ip, None, "success");
            let after = Utc::now();
            
            // Verify timestamp is within reasonable range
            prop_assert!(log.access_timestamp >= before);
            prop_assert!(log.access_timestamp <= after);
        }
    }
}
```

### Unit Testing Focus Areas

1. **IP Address Extraction**:
   - Test X-Forwarded-For header parsing
   - Test X-Real-IP header parsing
   - Test IPv6 address handling
   - Test multiple IPs in X-Forwarded-For (take first)

2. **Status Determination**:
   - Test success status for valid requests
   - Test failed status for invalid tokens
   - Test quota_exceeded for traffic limits
   - Test expired for expired packages
   - Test disabled for disabled accounts

3. **Filter Combinations**:
   - Test single filter (user_id only)
   - Test date range filter
   - Test status filter
   - Test all filters combined

4. **Pagination**:
   - Test first page
   - Test last page
   - Test page size limits
   - Test empty results

5. **Admin Authentication**:
   - Test valid admin token
   - Test invalid token
   - Test non-admin user token
   - Test missing token

### Frontend Testing

**Component Tests** (Vue Test Utils):
- Test filter input handling
- Test pagination controls
- Test status badge rendering
- Test timestamp formatting

**Integration Tests**:
- Test API call with filters
- Test pagination navigation
- Test filter clearing
- Test error handling

### Performance Testing

**Load Testing**:
- Simulate high-volume subscription access
- Verify logging doesn't impact response times
- Test database connection pool under load

**Query Performance**:
- Test date range queries with large datasets
- Verify index usage with EXPLAIN ANALYZE
- Test pagination performance with millions of records

### Migration Testing

**Migration Tests**:
- Test migration up (create table)
- Test migration down (drop table)
- Test idempotency (run migration twice)
- Verify indexes created
- Verify foreign keys created

## Implementation Notes

### Performance Considerations

1. **Async Logging**: Use `tokio::spawn` to avoid blocking subscription requests
2. **Connection Pooling**: Leverage existing SQLx connection pool
3. **Index Strategy**: Indexes on user_id, access_timestamp, and response_status for efficient filtering
4. **Pagination**: Use LIMIT/OFFSET for pagination (consider cursor-based for very large datasets)

### Security Considerations

1. **Admin-Only Access**: Verify admin status for all access log queries
2. **SQL Injection Prevention**: Use parameterized queries exclusively
3. **PII Protection**: Consider IP address as PII, implement appropriate access controls
4. **Rate Limiting**: Consider rate limiting on access log queries to prevent abuse

### Scalability Considerations

1. **Data Retention**: Implement log rotation/archival strategy for old logs
2. **Partitioning**: Consider table partitioning by date for very large datasets
3. **Archival**: Move old logs to cold storage after retention period
4. **Aggregation**: Consider pre-aggregating statistics for dashboard views

### Monitoring and Observability

1. **Metrics to Track**:
   - Access log write failures
   - Query response times
   - Failed access attempts by status
   - Top users by access count

2. **Alerts**:
   - High rate of logging failures
   - Unusual spike in failed access attempts
   - Slow query performance

3. **Logging**:
   - Log all logging failures with context
   - Log slow queries for optimization
   - Log admin access to logs for audit trail
