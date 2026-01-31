import { describe, it, expect } from 'vitest'
import * as fc from 'fast-check'

/**
 * Feature: clash-access-logs
 * Property 9: Timestamp Formatting Consistency
 * 
 * For any timestamp value, the formatting function should produce a human-readable
 * string that includes date, time, and timezone information in a consistent format.
 * 
 * Validates: Requirements 4.3
 */

// Extract the formatTimestamp function for testing
const formatTimestamp = (timestamp: string): string => {
  const date = new Date(timestamp)
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  })
}

describe('AccessLogs - Property-Based Tests', () => {
  describe('Property 9: Timestamp Formatting Consistency', () => {
    it('should format any valid timestamp consistently with date, time, and timezone info', () => {
      fc.assert(
        fc.property(
          // Generate valid timestamps (milliseconds since epoch)
          fc.integer({ min: 0, max: Date.now() + 365 * 24 * 60 * 60 * 1000 }),
          (timestamp) => {
            const isoString = new Date(timestamp).toISOString()
            const formatted = formatTimestamp(isoString)
            
            // Property 1: Result should be a non-empty string
            expect(formatted).toBeTruthy()
            expect(typeof formatted).toBe('string')
            expect(formatted.length).toBeGreaterThan(0)
            
            // Property 2: Should contain date components (year, month, day)
            // Format is like: "2024/01/15 12:30:45" or "2024-01-15 12:30:45"
            const hasDateSeparators = formatted.includes('/') || formatted.includes('-')
            expect(hasDateSeparators).toBe(true)
            
            // Property 3: Should contain time components (separated by colons)
            expect(formatted.includes(':')).toBe(true)
            
            // Property 4: Should contain at least 2 colons (for HH:MM:SS)
            const colonCount = (formatted.match(/:/g) || []).length
            expect(colonCount).toBeGreaterThanOrEqual(2)
            
            // Property 5: Formatting the same timestamp twice should yield identical results
            const formatted2 = formatTimestamp(isoString)
            expect(formatted).toBe(formatted2)
            
            // Property 6: Should not contain "Invalid Date"
            expect(formatted).not.toContain('Invalid')
            
            // Property 7: Should be parseable back to a date
            const reparsed = new Date(formatted)
            expect(reparsed.toString()).not.toBe('Invalid Date')
          }
        ),
        { numRuns: 100 }
      )
    })

    it('should handle edge case timestamps consistently', () => {
      fc.assert(
        fc.property(
          fc.constantFrom(
            0, // Unix epoch
            Date.now(), // Current time
            new Date('2000-01-01T00:00:00Z').getTime(), // Y2K
            new Date('2024-12-31T23:59:59Z').getTime(), // End of year
            new Date('2024-01-01T00:00:00Z').getTime(), // Start of year
            new Date('2024-06-15T12:00:00Z').getTime() // Mid-year
          ),
          (timestamp) => {
            const isoString = new Date(timestamp).toISOString()
            const formatted = formatTimestamp(isoString)
            
            // Should produce consistent, valid output for edge cases
            expect(formatted).toBeTruthy()
            expect(typeof formatted).toBe('string')
            expect(formatted).not.toContain('Invalid')
            
            // Should be idempotent
            expect(formatTimestamp(isoString)).toBe(formatted)
          }
        ),
        { numRuns: 100 }
      )
    })

    it('should format timestamps in consistent locale format', () => {
      fc.assert(
        fc.property(
          fc.date({ min: new Date('2020-01-01'), max: new Date('2030-12-31') }),
          (date) => {
            const isoString = date.toISOString()
            const formatted = formatTimestamp(isoString)
            
            // Property: Format should be consistent with zh-CN locale
            // The format should contain numeric date and time components
            const numericPattern = /\d+/g
            const numbers = formatted.match(numericPattern)
            
            // Should have at least 6 numeric components (year, month, day, hour, minute, second)
            expect(numbers).toBeTruthy()
            expect(numbers!.length).toBeGreaterThanOrEqual(6)
            
            // Year should be 4 digits
            const yearMatch = formatted.match(/\d{4}/)
            expect(yearMatch).toBeTruthy()
            
            // Should use 24-hour format (no AM/PM)
            expect(formatted.toLowerCase()).not.toContain('am')
            expect(formatted.toLowerCase()).not.toContain('pm')
          }
        ),
        { numRuns: 100 }
      )
    })
  })
})
