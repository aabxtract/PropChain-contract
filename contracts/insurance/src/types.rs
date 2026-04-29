// Data types for the insurance contract (Issue #101 - extracted from lib.rs)

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum PolicyStatus {
    Active,
    Expired,
    Cancelled,
    Claimed,
    Suspended,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum CoverageType {
    Fire,
    Flood,
    Earthquake,
    Theft,
    LiabilityDamage,
    NaturalDisaster,
    Comprehensive,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ClaimStatus {
    Pending,
    UnderReview,
    OracleVerifying,
    Approved,
    Rejected,
    Paid,
    Disputed,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum RiskLevel {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct InsurancePolicy {
    pub policy_id: u64,
    pub property_id: u64,
    pub policyholder: AccountId,
    pub coverage_type: CoverageType,
    pub coverage_amount: u128,
    pub premium_amount: u128,
    pub deductible: u128,
    pub start_time: u64,
    pub end_time: u64,
    pub status: PolicyStatus,
    pub risk_level: RiskLevel,
    pub pool_id: u64,
    pub claims_count: u32,
    pub total_claimed: u128,
    pub metadata_url: String,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct InsuranceClaim {
    pub claim_id: u64,
    pub policy_id: u64,
    pub claimant: AccountId,
    pub claim_amount: u128,
    pub description: String,
    pub evidence_url: String,
    pub oracle_report_url: String,
    pub status: ClaimStatus,
    pub submitted_at: u64,
    pub processed_at: Option<u64>,
    pub payout_amount: u128,
    pub assessor: Option<AccountId>,
    pub rejection_reason: String,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct RiskPool {
    pub pool_id: u64,
    pub name: String,
    pub coverage_type: CoverageType,
    pub total_capital: u128,
    pub available_capital: u128,
    pub total_premiums_collected: u128,
    pub total_claims_paid: u128,
    pub active_policies: u64,
    pub max_coverage_ratio: u32,
    pub reinsurance_threshold: u128,
    pub created_at: u64,
    pub is_active: bool,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct RiskAssessment {
    pub property_id: u64,
    pub location_risk_score: u32,
    pub construction_risk_score: u32,
    pub age_risk_score: u32,
    pub claims_history_score: u32,
    pub overall_risk_score: u32,
    pub risk_level: RiskLevel,
    pub assessed_at: u64,
    pub valid_until: u64,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PremiumCalculation {
    pub base_rate: u32,
    pub risk_multiplier: u32,
    pub coverage_multiplier: u32,
    pub annual_premium: u128,
    pub monthly_premium: u128,
    pub deductible: u128,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ReinsuranceAgreement {
    pub agreement_id: u64,
    pub reinsurer: AccountId,
    pub coverage_limit: u128,
    pub retention_limit: u128,
    pub premium_ceded_rate: u32,
    pub coverage_types: Vec<CoverageType>,
    pub start_time: u64,
    pub end_time: u64,
    pub is_active: bool,
    pub total_ceded_premiums: u128,
    pub total_recoveries: u128,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct InsuranceToken {
    pub token_id: u64,
    pub policy_id: u64,
    pub owner: AccountId,
    pub face_value: u128,
    pub is_tradeable: bool,
    pub created_at: u64,
    pub listed_price: Option<u128>,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ActuarialModel {
    pub model_id: u64,
    pub coverage_type: CoverageType,
    pub loss_frequency: u32,
    pub average_loss_severity: u128,
    pub expected_loss_ratio: u32,
    pub confidence_level: u32,
    pub last_updated: u64,
    pub data_points: u32,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct UnderwritingCriteria {
    pub max_property_age_years: u32,
    pub min_property_value: u128,
    pub max_property_value: u128,
    pub excluded_locations: Vec<String>,
    pub required_safety_features: bool,
    pub max_previous_claims: u32,
    pub min_risk_score: u32,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PoolLiquidityProvider {
    pub provider: AccountId,
    pub pool_id: u64,
    pub deposited_amount: u128,
    pub share_percentage: u32,
    pub deposited_at: u64,
    pub last_reward_claim: u64,
    pub accumulated_rewards: u128,
}

// =========================================================================
// CLAIM AUTOMATION (oracle-triggered parametric claims)
// =========================================================================

/// Metric type an oracle can report against. Units are policy-specific and
/// must match what the oracle is configured to publish (e.g. wind speed in
/// km/h, magnitude * 100, etc.).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum TriggerMetric {
    WindSpeed,
    FloodLevel,
    EarthquakeMagnitude,
    Temperature,
    Generic,
}

/// How the observed value is compared to the threshold.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum TriggerComparator {
    GreaterOrEqual,
    LessOrEqual,
}

/// How the gross claim amount is computed once a trigger fires.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum PayoutMode {
    /// Fixed payout amount, capped at remaining coverage.
    Fixed(u128),
    /// Percentage of remaining coverage, in basis points (0–10000).
    PercentBps(u32),
    /// Pay out the full remaining coverage.
    FullCoverage,
}

/// A condition registered against a policy. When an authorized oracle reports
/// a value that satisfies the comparator/threshold, a claim is automatically
/// created, approved, and paid.
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ClaimTrigger {
    pub trigger_id: u64,
    pub policy_id: u64,
    pub metric: TriggerMetric,
    pub comparator: TriggerComparator,
    pub threshold: u128,
    pub payout_mode: PayoutMode,
    pub is_active: bool,
    pub triggered: bool,
    pub last_observed_value: Option<u128>,
    pub last_report_url: String,
    pub created_at: u64,
    pub triggered_at: Option<u64>,
    pub triggering_claim_id: Option<u64>,
}
