//! Devnet IDL instruction coverage manifest.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevnetInstructionStatus {
    Implemented,
    PublicFlowPlanned,
    AdminOnlyPlanned,
    IntentionallyUnsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevnetInstructionCoverage {
    pub name: &'static str,
    pub status: DevnetInstructionStatus,
    pub notes: &'static str,
}

pub const DEVNET_IDL_ADDRESS: &str = "6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J";
pub const DEVNET_DOCS_IDL_VERSION: &str = "1.5.2";
pub const DEVNET_PR_IDL_VERSION: &str = "1.5.5";

pub const DEVNET_INSTRUCTION_COVERAGE: &[DevnetInstructionCoverage] = &[
    DevnetInstructionCoverage {
        name: "audit_trade_result",
        status: DevnetInstructionStatus::PublicFlowPlanned,
        notes: "trading settlement flow is not yet exposed as a high-level SDK API",
    },
    DevnetInstructionCoverage {
        name: "claim_batch_legacy",
        status: DevnetInstructionStatus::PublicFlowPlanned,
        notes: "claim/refund flows remain planned",
    },
    DevnetInstructionCoverage {
        name: "claim_via_resolution",
        status: DevnetInstructionStatus::PublicFlowPlanned,
        notes: "resolution-claim flow remains planned",
    },
    DevnetInstructionCoverage {
        name: "close_intent",
        status: DevnetInstructionStatus::PublicFlowPlanned,
        notes: "trading intent lifecycle remains planned",
    },
    DevnetInstructionCoverage {
        name: "close_pricing_matrix",
        status: DevnetInstructionStatus::AdminOnlyPlanned,
        notes: "admin-only pricing matrix management",
    },
    DevnetInstructionCoverage {
        name: "create_intent",
        status: DevnetInstructionStatus::PublicFlowPlanned,
        notes: "trading intent lifecycle remains planned",
    },
    DevnetInstructionCoverage {
        name: "create_trade",
        status: DevnetInstructionStatus::PublicFlowPlanned,
        notes: "direct trade creation remains planned",
    },
    DevnetInstructionCoverage {
        name: "execute_match",
        status: DevnetInstructionStatus::PublicFlowPlanned,
        notes: "order matching remains planned",
    },
    DevnetInstructionCoverage {
        name: "expose_structs",
        status: DevnetInstructionStatus::IntentionallyUnsupported,
        notes: "IDL/type exposure helper, not an end-user flow",
    },
    DevnetInstructionCoverage {
        name: "initialize_pricing_matrix",
        status: DevnetInstructionStatus::AdminOnlyPlanned,
        notes: "admin-only pricing matrix management",
    },
    DevnetInstructionCoverage {
        name: "initialize_treasury_v2",
        status: DevnetInstructionStatus::AdminOnlyPlanned,
        notes: "admin-only treasury setup",
    },
    DevnetInstructionCoverage {
        name: "initialize_usdt_treasury",
        status: DevnetInstructionStatus::AdminOnlyPlanned,
        notes: "admin-only treasury setup",
    },
    DevnetInstructionCoverage {
        name: "insert_batch_root",
        status: DevnetInstructionStatus::AdminOnlyPlanned,
        notes: "oracle root insertion is not exposed to casual SDK users",
    },
    DevnetInstructionCoverage {
        name: "insert_fixtures_root",
        status: DevnetInstructionStatus::AdminOnlyPlanned,
        notes: "oracle root insertion is not exposed to casual SDK users",
    },
    DevnetInstructionCoverage {
        name: "insert_scores_root",
        status: DevnetInstructionStatus::AdminOnlyPlanned,
        notes: "oracle root insertion is not exposed to casual SDK users",
    },
    DevnetInstructionCoverage {
        name: "publish_resolution_root",
        status: DevnetInstructionStatus::AdminOnlyPlanned,
        notes: "oracle resolution root publishing is admin-only",
    },
    DevnetInstructionCoverage {
        name: "purchase_subscription_token_usdt",
        status: DevnetInstructionStatus::Implemented,
        notes: "typed builder and quote transaction safety checks are implemented",
    },
    DevnetInstructionCoverage {
        name: "refund_batch",
        status: DevnetInstructionStatus::PublicFlowPlanned,
        notes: "batch refunds remain planned",
    },
    DevnetInstructionCoverage {
        name: "request_devnet_faucet",
        status: DevnetInstructionStatus::Implemented,
        notes: "typed builder accepts an explicit faucet tracker account; automatic PDA derivation is not published in the IDL",
    },
    DevnetInstructionCoverage {
        name: "settle_matched_trade",
        status: DevnetInstructionStatus::PublicFlowPlanned,
        notes: "matched trade settlement remains planned",
    },
    DevnetInstructionCoverage {
        name: "settle_trade",
        status: DevnetInstructionStatus::PublicFlowPlanned,
        notes: "direct trade settlement remains planned",
    },
    DevnetInstructionCoverage {
        name: "subscribe",
        status: DevnetInstructionStatus::Implemented,
        notes: "subscription transaction builder and setup flow are implemented",
    },
    DevnetInstructionCoverage {
        name: "update_pricing_matrix",
        status: DevnetInstructionStatus::AdminOnlyPlanned,
        notes: "admin-only pricing matrix management",
    },
    DevnetInstructionCoverage {
        name: "validate_fixture",
        status: DevnetInstructionStatus::Implemented,
        notes: "typed instruction builder and simulation helper are implemented",
    },
    DevnetInstructionCoverage {
        name: "validate_fixture_batch",
        status: DevnetInstructionStatus::Implemented,
        notes: "typed instruction builder and simulation helper are implemented",
    },
    DevnetInstructionCoverage {
        name: "validate_odds",
        status: DevnetInstructionStatus::Implemented,
        notes: "typed instruction builder and simulation helper are implemented",
    },
    DevnetInstructionCoverage {
        name: "validate_stat",
        status: DevnetInstructionStatus::Implemented,
        notes: "typed instruction builder and simulation helper are implemented",
    },
    DevnetInstructionCoverage {
        name: "validate_stat_v2",
        status: DevnetInstructionStatus::Implemented,
        notes: "typed instruction builder and simulation helper are implemented for the PR Devnet IDL",
    },
    DevnetInstructionCoverage {
        name: "withdraw_usdt",
        status: DevnetInstructionStatus::AdminOnlyPlanned,
        notes: "admin-only treasury withdrawal",
    },
];

pub fn devnet_instruction_coverage() -> &'static [DevnetInstructionCoverage] {
    DEVNET_INSTRUCTION_COVERAGE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_lists_every_known_devnet_instruction_once() {
        let expected = [
            "audit_trade_result",
            "claim_batch_legacy",
            "claim_via_resolution",
            "close_intent",
            "close_pricing_matrix",
            "create_intent",
            "create_trade",
            "execute_match",
            "expose_structs",
            "initialize_pricing_matrix",
            "initialize_treasury_v2",
            "initialize_usdt_treasury",
            "insert_batch_root",
            "insert_fixtures_root",
            "insert_scores_root",
            "publish_resolution_root",
            "purchase_subscription_token_usdt",
            "refund_batch",
            "request_devnet_faucet",
            "settle_matched_trade",
            "settle_trade",
            "subscribe",
            "update_pricing_matrix",
            "validate_fixture",
            "validate_fixture_batch",
            "validate_odds",
            "validate_stat",
            "validate_stat_v2",
            "withdraw_usdt",
        ];
        let names = DEVNET_INSTRUCTION_COVERAGE
            .iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        assert_eq!(names, expected);
    }
}
