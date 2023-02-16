//! Task-level budget ledger (Living `13` §5).
//!
//! Maps [`zb_types::GenerateLimits`] onto Acorn [`Budget`] and [`CodecLimits`]
//! without giving every member a fresh default 64 MiB quota.

use acorn_codec::CodecLimits;
use acorn_core::Budget;
use zb_types::{BudgetUsage, GenerateLimits, ZbError, ZbResult};

/// Running quotas for one generate / verify task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBudgetLedger {
    limits: GenerateLimits,
    remaining_decoded: u64,
    remaining_read: u64,
    members_decoded: u64,
}

impl TaskBudgetLedger {
    /// Start a ledger from task limits.
    pub fn new(limits: GenerateLimits) -> Self {
        let remaining_decoded = limits.max_decoded_bytes;
        let remaining_read = limits.max_live_memory_bytes;
        Self { limits, remaining_decoded, remaining_read, members_decoded: 0 }
    }

    /// Ledger using the shared small-sample profile.
    pub fn small_sample() -> Self {
        Self::new(GenerateLimits::small_sample())
    }

    /// Remaining decoded-byte quota.
    pub fn remaining_decoded(&self) -> u64 {
        self.remaining_decoded
    }

    /// Remaining read / live-memory quota used for Acorn scans and payload reads.
    pub fn remaining_read(&self) -> u64 {
        self.remaining_read
    }

    /// Snapshot charged usage for `zb.report/v0`.
    pub fn usage(&self) -> BudgetUsage {
        BudgetUsage {
            max_decoded_bytes: self.limits.max_decoded_bytes,
            decoded_bytes_used: self.limits.max_decoded_bytes - self.remaining_decoded,
            max_live_memory_bytes: self.limits.max_live_memory_bytes,
            read_bytes_used: self.limits.max_live_memory_bytes - self.remaining_read,
            members_decoded: self.members_decoded,
        }
    }

    /// Reserve an Acorn parse budget for one ZIP member decode.
    pub fn reserve_member_decode(
        &mut self,
        archive_len: u64,
        compressed_size: u64,
        uncompressed_size: u64,
    ) -> ZbResult<Budget> {
        if self.members_decoded >= self.limits.max_members as u64 {
            return Err(ZbError::resource_limited("member decode budget exhausted"));
        }
        if uncompressed_size > self.remaining_decoded {
            return Err(ZbError::resource_limited("remaining max_decoded_bytes insufficient for member"));
        }
        let read_need = archive_len.max(compressed_size);
        if read_need > self.remaining_read {
            return Err(ZbError::resource_limited("remaining live-memory read budget insufficient for member"));
        }
        let cache_cap =
            self.limits.max_live_memory_bytes.min(self.remaining_read).min(archive_len.saturating_mul(2).max(compressed_size));
        Ok(Budget {
            max_read_bytes: read_need,
            max_decoded_bytes: uncompressed_size,
            max_envelope_depth: self.limits.max_depth,
            max_container_members: (self.limits.max_members as u64).saturating_sub(self.members_decoded),
            max_nodes: self.limits.max_nodes as u64,
            max_references: self.limits.max_edges as u64 * 4,
            max_pointer_hops: self.limits.max_depth as u64 * 8,
            max_scan_bytes: archive_len.min(8 * 1024 * 1024),
            max_cache_bytes: cache_cap,
        })
    }

    /// Record actual decode consumption for a member.
    pub fn settle_member_decode(&mut self, decoded_size: u64, read_bytes: u64) -> ZbResult<()> {
        if decoded_size > self.remaining_decoded {
            return Err(ZbError::resource_limited("member decode exceeded remaining max_decoded_bytes"));
        }
        if read_bytes > self.remaining_read {
            return Err(ZbError::resource_limited("member read exceeded remaining live-memory budget"));
        }
        self.remaining_decoded -= decoded_size;
        self.remaining_read -= read_bytes;
        self.members_decoded += 1;
        Ok(())
    }

    /// Reserve codec limits for a raw DEFLATE self-check.
    pub fn reserve_raw_deflate(&mut self, input_len: u64, max_output: u64) -> ZbResult<CodecLimits> {
        let output_cap = max_output.min(self.remaining_decoded);
        if max_output > 0 && output_cap == 0 {
            return Err(ZbError::resource_limited("remaining max_decoded_bytes insufficient for raw deflate"));
        }
        if input_len > self.remaining_read {
            return Err(ZbError::resource_limited("remaining read budget insufficient for raw deflate input"));
        }
        Ok(CodecLimits::new().with_max_input_bytes(input_len).with_max_output_bytes(output_cap))
    }

    /// Record raw DEFLATE decode consumption.
    pub fn settle_raw_deflate(&mut self, decoded_size: u64, input_len: u64) -> ZbResult<()> {
        if decoded_size > self.remaining_decoded {
            return Err(ZbError::resource_limited("raw deflate exceeded remaining max_decoded_bytes"));
        }
        if input_len > self.remaining_read {
            return Err(ZbError::resource_limited("raw deflate exceeded remaining read budget"));
        }
        self.remaining_decoded -= decoded_size;
        self.remaining_read -= input_len;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zb_types::ErrorCode;

    #[test]
    fn member_decode_shares_one_task_quota() {
        let limits = GenerateLimits {
            max_members: 2,
            max_decoded_bytes: 100,
            max_live_memory_bytes: 200,
            ..GenerateLimits::small_sample()
        };
        let mut ledger = TaskBudgetLedger::new(limits);
        let budget = ledger.reserve_member_decode(50, 40, 60).expect("first member fits");
        assert_eq!(budget.max_decoded_bytes, 60);
        ledger.settle_member_decode(60, 40).expect("first member settles");
        assert_eq!(ledger.remaining_decoded(), 40);
        ledger.reserve_member_decode(50, 30, 30).expect("second member fits");
        ledger.settle_member_decode(30, 30).expect("second member settles");
        let err = ledger.reserve_member_decode(50, 10, 10).expect_err("third member exceeds max_members");
        assert_eq!(err.code, ErrorCode::ResourceLimited);
    }

    #[test]
    fn hundred_members_do_not_each_get_default_budget() {
        let limits = GenerateLimits {
            max_members: 100,
            max_decoded_bytes: 128 * 1024,
            max_live_memory_bytes: 256 * 1024,
            ..GenerateLimits::small_sample()
        };
        let mut ledger = TaskBudgetLedger::new(limits);
        for _ in 0..100 {
            let budget = ledger.reserve_member_decode(2048, 512, 1024).expect("member fits under shared ledger");
            assert_eq!(budget.max_decoded_bytes, 1024);
            assert!(budget.max_decoded_bytes < 64 * 1024 * 1024);
            ledger.settle_member_decode(1024, 512).expect("member settles");
        }
        assert_eq!(ledger.usage().members_decoded, 100);
    }

    #[test]
    fn raw_deflate_respects_remaining_decode_quota() {
        let limits = GenerateLimits { max_decoded_bytes: 32, max_live_memory_bytes: 64, ..GenerateLimits::small_sample() };
        let mut ledger = TaskBudgetLedger::new(limits);
        let codec = ledger.reserve_raw_deflate(16, 24).expect("first deflate fits");
        assert_eq!(codec.max_output_bytes, Some(24));
        ledger.settle_raw_deflate(24, 16).expect("first deflate settles");
        assert_eq!(ledger.remaining_decoded(), 8);
        let capped = ledger.reserve_raw_deflate(8, 16).expect("second deflate is capped to remaining quota");
        assert_eq!(capped.max_output_bytes, Some(8));
        ledger.settle_raw_deflate(8, 8).expect("second deflate settles");
        let err = ledger.reserve_raw_deflate(4, 4).expect_err("third deflate exceeds exhausted decode quota");
        assert_eq!(err.code, ErrorCode::ResourceLimited);
    }
}
