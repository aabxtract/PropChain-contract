// Unit tests for the insurance contract (Issue #101 - extracted from lib.rs)

#[cfg(test)]
mod insurance_tests {
    use ink::env::{test, DefaultEnvironment};

    use crate::propchain_insurance::{
        ClaimStatus, CoverageType, InsuranceError, PayoutMode, PolicyStatus, PropertyInsurance,
        TriggerComparator, TriggerMetric,
    };

    fn setup() -> PropertyInsurance {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        // Start at 35 days so `now - last_claim(0) > 30-day cooldown`
        test::set_block_timestamp::<DefaultEnvironment>(3_000_000);
        PropertyInsurance::new(accounts.alice)
    }

    fn add_risk_assessment(contract: &mut PropertyInsurance, property_id: u64) {
        contract
            .update_risk_assessment(property_id, 75, 80, 85, 90, 86_400 * 365)
            .expect("risk assessment failed");
    }

    fn create_pool(contract: &mut PropertyInsurance) -> u64 {
        contract
            .create_risk_pool(
                "Fire & Flood Pool".into(),
                CoverageType::Fire,
                8000,
                500_000_000_000u128,
            )
            .expect("pool creation failed")
    }

    // =========================================================================
    // CONSTRUCTOR
    // =========================================================================

    #[ink::test]
    fn test_new_contract_initialised() {
        let contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        assert_eq!(contract.get_admin(), accounts.alice);
        assert_eq!(contract.get_policy_count(), 0);
        assert_eq!(contract.get_claim_count(), 0);
    }

    // =========================================================================
    // POOL TESTS
    // =========================================================================

    #[ink::test]
    fn test_create_risk_pool_works() {
        let mut contract = setup();
        let pool_id = create_pool(&mut contract);
        assert_eq!(pool_id, 1);
        let pool = contract.get_pool(1).unwrap();
        assert_eq!(pool.pool_id, 1);
        assert!(pool.is_active);
        assert_eq!(pool.active_policies, 0);
    }

    #[ink::test]
    fn test_create_risk_pool_unauthorized() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.create_risk_pool(
            "Unauthorized Pool".into(),
            CoverageType::Fire,
            8000,
            1_000_000,
        );
        assert_eq!(result, Err(InsuranceError::Unauthorized));
    }

    #[ink::test]
    fn test_provide_pool_liquidity_works() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(1_000_000_000_000u128);
        let result = contract.provide_pool_liquidity(pool_id);
        assert!(result.is_ok());
        let pool = contract.get_pool(pool_id).unwrap();
        assert_eq!(pool.total_capital, 1_000_000_000_000u128);
        assert_eq!(pool.available_capital, 1_000_000_000_000u128);
    }

    #[ink::test]
    fn test_provide_liquidity_nonexistent_pool_fails() {
        let mut contract = setup();
        test::set_value_transferred::<DefaultEnvironment>(1_000_000u128);
        let result = contract.provide_pool_liquidity(999);
        assert_eq!(result, Err(InsuranceError::PoolNotFound));
    }

    // =========================================================================
    // RISK ASSESSMENT TESTS
    // =========================================================================

    #[ink::test]
    fn test_update_risk_assessment_works() {
        let mut contract = setup();
        add_risk_assessment(&mut contract, 1);
        let assessment = contract.get_risk_assessment(1).unwrap();
        assert_eq!(assessment.property_id, 1);
        assert_eq!(assessment.overall_risk_score, 82); // (75+80+85+90)/4
        assert!(assessment.valid_until > 0);
    }

    #[ink::test]
    fn test_risk_assessment_unauthorized() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.update_risk_assessment(1, 70, 70, 70, 70, 86400);
        assert_eq!(result, Err(InsuranceError::Unauthorized));
    }

    #[ink::test]
    fn test_authorized_oracle_can_assess() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        contract.authorize_oracle(accounts.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.update_risk_assessment(1, 70, 70, 70, 70, 86400);
        assert!(result.is_ok());
    }

    // =========================================================================
    // PREMIUM CALCULATION TESTS
    // =========================================================================

    #[ink::test]
    fn test_calculate_premium_works() {
        let mut contract = setup();
        add_risk_assessment(&mut contract, 1);
        let result = contract.calculate_premium(1, 1_000_000_000_000u128, CoverageType::Fire);
        assert!(result.is_ok());
        let calc = result.unwrap();
        assert!(calc.annual_premium > 0);
        assert!(calc.monthly_premium > 0);
        assert!(calc.deductible > 0);
        assert_eq!(calc.base_rate, 150);
    }

    #[ink::test]
    fn test_premium_without_assessment_fails() {
        let contract = setup();
        let result = contract.calculate_premium(999, 1_000_000u128, CoverageType::Fire);
        assert_eq!(result, Err(InsuranceError::PropertyNotInsurable));
    }

    #[ink::test]
    fn test_comprehensive_coverage_higher_premium() {
        let mut contract = setup();
        add_risk_assessment(&mut contract, 1);
        let fire_calc = contract
            .calculate_premium(1, 1_000_000_000_000u128, CoverageType::Fire)
            .unwrap();
        let comp_calc = contract
            .calculate_premium(1, 1_000_000_000_000u128, CoverageType::Comprehensive)
            .unwrap();
        assert!(comp_calc.annual_premium > fire_calc.annual_premium);
    }

    // =========================================================================
    // POLICY CREATION TESTS
    // =========================================================================

    #[ink::test]
    fn test_create_policy_works() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);

        let calc = contract
            .calculate_premium(1, 500_000_000_000u128, CoverageType::Fire)
            .unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);

        let result = contract.create_policy(
            1,
            CoverageType::Fire,
            500_000_000_000u128,
            pool_id,
            86_400 * 365,
            "ipfs://policy-metadata".into(),
        );
        assert!(result.is_ok());

        let policy_id = result.unwrap();
        let policy = contract.get_policy(policy_id).unwrap();
        assert_eq!(policy.property_id, 1);
        assert_eq!(policy.policyholder, accounts.bob);
        assert_eq!(policy.status, PolicyStatus::Active);
        assert_eq!(contract.get_policy_count(), 1);
    }

    #[ink::test]
    fn test_create_policy_insufficient_premium_fails() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(1u128);
        let result = contract.create_policy(
            1,
            CoverageType::Fire,
            500_000_000_000u128,
            pool_id,
            86_400 * 365,
            "ipfs://policy-metadata".into(),
        );
        assert_eq!(result, Err(InsuranceError::InsufficientPremium));
    }

    #[ink::test]
    fn test_create_policy_nonexistent_pool_fails() {
        let mut contract = setup();
        add_risk_assessment(&mut contract, 1);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(1_000_000_000_000u128);
        let result = contract.create_policy(
            1,
            CoverageType::Fire,
            100_000u128,
            999,
            86_400 * 365,
            "ipfs://policy-metadata".into(),
        );
        assert_eq!(result, Err(InsuranceError::PoolNotFound));
    }

    // =========================================================================
    // POLICY CANCELLATION TESTS
    // =========================================================================

    #[ink::test]
    fn test_cancel_policy_by_policyholder() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        let calc = contract
            .calculate_premium(1, 500_000_000_000u128, CoverageType::Fire)
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        let policy_id = contract
            .create_policy(
                1,
                CoverageType::Fire,
                500_000_000_000u128,
                pool_id,
                86_400 * 365,
                "ipfs://test".into(),
            )
            .unwrap();
        let result = contract.cancel_policy(policy_id);
        assert!(result.is_ok());
        let policy = contract.get_policy(policy_id).unwrap();
        assert_eq!(policy.status, PolicyStatus::Cancelled);
    }

    #[ink::test]
    fn test_cancel_policy_by_non_owner_fails() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        let calc = contract
            .calculate_premium(1, 500_000_000_000u128, CoverageType::Fire)
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        let policy_id = contract
            .create_policy(
                1,
                CoverageType::Fire,
                500_000_000_000u128,
                pool_id,
                86_400 * 365,
                "ipfs://test".into(),
            )
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let result = contract.cancel_policy(policy_id);
        assert_eq!(result, Err(InsuranceError::Unauthorized));
    }

    // =========================================================================
    // CLAIM SUBMISSION TESTS
    // =========================================================================

    #[ink::test]
    fn test_submit_claim_works() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        let calc = contract
            .calculate_premium(1, 500_000_000_000u128, CoverageType::Fire)
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        let policy_id = contract
            .create_policy(
                1,
                CoverageType::Fire,
                500_000_000_000u128,
                pool_id,
                86_400 * 365,
                "ipfs://test".into(),
            )
            .unwrap();
        let result = contract.submit_claim(
            policy_id,
            10_000_000_000u128,
            "Fire damage to property".into(),
            "ipfs://evidence123".into(),
        );
        assert!(result.is_ok());
        let claim_id = result.unwrap();
        let claim = contract.get_claim(claim_id).unwrap();
        assert_eq!(claim.policy_id, policy_id);
        assert_eq!(claim.claimant, accounts.bob);
        assert_eq!(claim.status, ClaimStatus::Pending);
        assert_eq!(contract.get_claim_count(), 1);
    }

    #[ink::test]
    fn test_claim_exceeds_coverage_fails() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        let coverage = 500_000_000_000u128;
        let calc = contract
            .calculate_premium(1, coverage, CoverageType::Fire)
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        let policy_id = contract
            .create_policy(
                1,
                CoverageType::Fire,
                coverage,
                pool_id,
                86_400 * 365,
                "ipfs://test".into(),
            )
            .unwrap();
        let result = contract.submit_claim(
            policy_id,
            coverage * 2,
            "Huge fire".into(),
            "ipfs://evidence".into(),
        );
        assert_eq!(result, Err(InsuranceError::ClaimExceedsCoverage));
    }

    #[ink::test]
    fn test_claim_by_nonpolicyholder_fails() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        let calc = contract
            .calculate_premium(1, 500_000_000_000u128, CoverageType::Fire)
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        let policy_id = contract
            .create_policy(
                1,
                CoverageType::Fire,
                500_000_000_000u128,
                pool_id,
                86_400 * 365,
                "ipfs://test".into(),
            )
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let result = contract.submit_claim(
            policy_id,
            1_000u128,
            "Fraud attempt".into(),
            "ipfs://x".into(),
        );
        assert_eq!(result, Err(InsuranceError::Unauthorized));
    }

    // =========================================================================
    // CLAIM PROCESSING TESTS
    // =========================================================================

    #[ink::test]
    fn test_process_claim_approve_works() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        let coverage = 500_000_000_000u128;
        let calc = contract
            .calculate_premium(1, coverage, CoverageType::Fire)
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        let policy_id = contract
            .create_policy(
                1,
                CoverageType::Fire,
                coverage,
                pool_id,
                86_400 * 365,
                "ipfs://test".into(),
            )
            .unwrap();
        let claim_id = contract
            .submit_claim(
                policy_id,
                10_000_000_000u128,
                "Fire damage".into(),
                "ipfs://evidence".into(),
            )
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result =
            contract.process_claim(claim_id, true, "ipfs://oracle-report".into(), String::new());
        assert!(result.is_ok());
        let claim = contract.get_claim(claim_id).unwrap();
        assert_eq!(claim.status, ClaimStatus::Paid);
        assert!(claim.payout_amount > 0);
    }

    #[ink::test]
    fn test_process_claim_reject_works() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        let calc = contract
            .calculate_premium(1, 500_000_000_000u128, CoverageType::Fire)
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        let policy_id = contract
            .create_policy(
                1,
                CoverageType::Fire,
                500_000_000_000u128,
                pool_id,
                86_400 * 365,
                "ipfs://test".into(),
            )
            .unwrap();
        let claim_id = contract
            .submit_claim(
                policy_id,
                5_000_000_000u128,
                "Fraudulent claim".into(),
                "ipfs://fake-evidence".into(),
            )
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.process_claim(
            claim_id,
            false,
            "ipfs://oracle-report".into(),
            "Evidence does not support claim".into(),
        );
        assert!(result.is_ok());
        let claim = contract.get_claim(claim_id).unwrap();
        assert_eq!(claim.status, ClaimStatus::Rejected);
    }

    #[ink::test]
    fn test_process_claim_unauthorized_fails() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        let calc = contract
            .calculate_premium(1, 500_000_000_000u128, CoverageType::Fire)
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        let policy_id = contract
            .create_policy(
                1,
                CoverageType::Fire,
                500_000_000_000u128,
                pool_id,
                86_400 * 365,
                "ipfs://test".into(),
            )
            .unwrap();
        let claim_id = contract
            .submit_claim(policy_id, 1_000_000u128, "Damage".into(), "ipfs://e".into())
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let result = contract.process_claim(claim_id, true, "ipfs://r".into(), String::new());
        assert_eq!(result, Err(InsuranceError::Unauthorized));
    }

    #[ink::test]
    fn test_authorized_assessor_can_process_claim() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        let calc = contract
            .calculate_premium(1, 500_000_000_000u128, CoverageType::Fire)
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        let policy_id = contract
            .create_policy(
                1,
                CoverageType::Fire,
                500_000_000_000u128,
                pool_id,
                86_400 * 365,
                "ipfs://test".into(),
            )
            .unwrap();
        let claim_id = contract
            .submit_claim(policy_id, 1_000_000u128, "Damage".into(), "ipfs://e".into())
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.authorize_assessor(accounts.charlie).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let result = contract.process_claim(
            claim_id,
            false,
            "ipfs://r".into(),
            "Insufficient evidence".into(),
        );
        assert!(result.is_ok());
    }

    // =========================================================================
    // REINSURANCE TESTS
    // =========================================================================

    #[ink::test]
    fn test_register_reinsurance_works() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let result = contract.register_reinsurance(
            accounts.bob,
            10_000_000_000_000u128,
            500_000_000_000u128,
            2000,
            [CoverageType::Fire, CoverageType::Flood].to_vec(),
            86_400 * 365,
        );
        assert!(result.is_ok());
        let agreement_id = result.unwrap();
        let agreement = contract.get_reinsurance_agreement(agreement_id).unwrap();
        assert_eq!(agreement.reinsurer, accounts.bob);
        assert!(agreement.is_active);
    }

    #[ink::test]
    fn test_register_reinsurance_unauthorized_fails() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.register_reinsurance(
            accounts.bob,
            1_000_000u128,
            100_000u128,
            2000,
            [CoverageType::Fire].to_vec(),
            86_400,
        );
        assert_eq!(result, Err(InsuranceError::Unauthorized));
    }

    // =========================================================================
    // TOKEN / SECONDARY MARKET TESTS
    // =========================================================================

    #[ink::test]
    fn test_token_minted_on_policy_creation() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        let calc = contract
            .calculate_premium(1, 500_000_000_000u128, CoverageType::Fire)
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        let policy_id = contract
            .create_policy(
                1,
                CoverageType::Fire,
                500_000_000_000u128,
                pool_id,
                86_400 * 365,
                "ipfs://test".into(),
            )
            .unwrap();
        let token = contract.get_token(1).unwrap();
        assert_eq!(token.policy_id, policy_id);
        assert_eq!(token.owner, accounts.bob);
        assert!(token.is_tradeable);
    }

    #[ink::test]
    fn test_list_and_purchase_token() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        let calc = contract
            .calculate_premium(1, 500_000_000_000u128, CoverageType::Fire)
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        contract
            .create_policy(
                1,
                CoverageType::Fire,
                500_000_000_000u128,
                pool_id,
                86_400 * 365,
                "ipfs://test".into(),
            )
            .unwrap();
        // Bob lists token 1
        assert!(contract.list_token_for_sale(1, 100_000_000u128).is_ok());
        assert!(contract.get_token_listings().contains(&1));
        // Charlie buys token
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        test::set_value_transferred::<DefaultEnvironment>(100_000_000u128);
        assert!(contract.purchase_token(1).is_ok());
        let token = contract.get_token(1).unwrap();
        assert_eq!(token.owner, accounts.charlie);
        assert!(token.listed_price.is_none());
        let policy = contract.get_policy(1).unwrap();
        assert_eq!(policy.policyholder, accounts.charlie);
    }

    // =========================================================================
    // ACTUARIAL MODEL TESTS
    // =========================================================================

    #[ink::test]
    fn test_update_actuarial_model_works() {
        let mut contract = setup();
        let result =
            contract.update_actuarial_model(CoverageType::Fire, 50, 50_000_000u128, 4500, 95, 1000);
        assert!(result.is_ok());
        let model = contract.get_actuarial_model(result.unwrap()).unwrap();
        assert_eq!(model.loss_frequency, 50);
        assert_eq!(model.confidence_level, 95);
    }

    // =========================================================================
    // UNDERWRITING TESTS
    // =========================================================================

    #[ink::test]
    fn test_set_underwriting_criteria_works() {
        let mut contract = setup();
        let pool_id = create_pool(&mut contract);
        let result = contract.set_underwriting_criteria(
            pool_id,
            50,
            10_000_000u128,
            1_000_000_000_000_000u128,
            true,
            3,
            40,
        );
        assert!(result.is_ok());
        let criteria = contract.get_underwriting_criteria(pool_id).unwrap();
        assert_eq!(criteria.max_property_age_years, 50);
        assert_eq!(criteria.max_previous_claims, 3);
        assert_eq!(criteria.min_risk_score, 40);
    }

    // =========================================================================
    // ADMIN TESTS
    // =========================================================================

    #[ink::test]
    fn test_set_platform_fee_works() {
        let mut contract = setup();
        assert!(contract.set_platform_fee_rate(300).is_ok());
    }

    #[ink::test]
    fn test_set_platform_fee_exceeds_max_fails() {
        let mut contract = setup();
        assert_eq!(
            contract.set_platform_fee_rate(1001),
            Err(InsuranceError::InvalidParameters)
        );
    }

    #[ink::test]
    fn test_set_claim_cooldown_works() {
        let mut contract = setup();
        assert!(contract.set_claim_cooldown(86_400).is_ok());
    }

    #[ink::test]
    fn test_authorize_oracle_and_assessor() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        assert!(contract.authorize_oracle(accounts.bob).is_ok());
        assert!(contract.authorize_assessor(accounts.charlie).is_ok());
    }

    // =========================================================================
    // LIQUIDITY PROVIDER TESTS
    // =========================================================================

    #[ink::test]
    fn test_liquidity_provider_tracking() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(5_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        let provider = contract
            .get_liquidity_provider(pool_id, accounts.bob)
            .unwrap();
        assert_eq!(provider.deposited_amount, 5_000_000_000_000u128);
        assert_eq!(provider.pool_id, pool_id);
    }

    // =========================================================================
    // QUERY TESTS
    // =========================================================================

    #[ink::test]
    fn test_get_policies_for_property() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        let calc = contract
            .calculate_premium(1, 500_000_000_000u128, CoverageType::Fire)
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 4);
        contract
            .create_policy(
                1,
                CoverageType::Fire,
                100_000_000_000u128,
                pool_id,
                86_400 * 365,
                "ipfs://p1".into(),
            )
            .unwrap();
        contract
            .create_policy(
                1,
                CoverageType::Theft,
                100_000_000_000u128,
                pool_id,
                86_400 * 365,
                "ipfs://p2".into(),
            )
            .unwrap();
        let property_policies = contract.get_property_policies(1);
        assert_eq!(property_policies.len(), 2);
    }

    #[ink::test]
    fn test_get_policyholder_policies() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        add_risk_assessment(&mut contract, 2);
        let calc1 = contract
            .calculate_premium(1, 100_000_000_000u128, CoverageType::Fire)
            .unwrap();
        let calc2 = contract
            .calculate_premium(2, 100_000_000_000u128, CoverageType::Flood)
            .unwrap();
        let total = (calc1.annual_premium + calc2.annual_premium) * 2;
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(total);
        contract
            .create_policy(
                1,
                CoverageType::Fire,
                100_000_000_000u128,
                pool_id,
                86_400 * 365,
                "ipfs://p1".into(),
            )
            .unwrap();
        contract
            .create_policy(
                2,
                CoverageType::Flood,
                100_000_000_000u128,
                pool_id,
                86_400 * 365,
                "ipfs://p2".into(),
            )
            .unwrap();
        let holder_policies = contract.get_policyholder_policies(accounts.bob);
        assert_eq!(holder_policies.len(), 2);
    }

    // =========================================================================
    // CLAIM AUTOMATION TESTS
    // =========================================================================

    /// Helper: bob has an active fire policy on property 1 with 500B coverage.
    /// Returns (contract, policy_id, coverage_amount).
    fn setup_with_active_policy() -> (PropertyInsurance, u64, u128) {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let pool_id = create_pool(&mut contract);
        test::set_value_transferred::<DefaultEnvironment>(10_000_000_000_000u128);
        contract.provide_pool_liquidity(pool_id).unwrap();
        add_risk_assessment(&mut contract, 1);
        let coverage = 500_000_000_000u128;
        let calc = contract
            .calculate_premium(1, coverage, CoverageType::Fire)
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        let policy_id = contract
            .create_policy(
                1,
                CoverageType::Fire,
                coverage,
                pool_id,
                86_400 * 365,
                "ipfs://test".into(),
            )
            .unwrap();
        (contract, policy_id, coverage)
    }

    #[ink::test]
    fn test_register_trigger_works() {
        let (mut contract, policy_id, _) = setup_with_active_policy();
        let trigger_id = contract
            .register_claim_trigger(
                policy_id,
                TriggerMetric::WindSpeed,
                TriggerComparator::GreaterOrEqual,
                200,
                PayoutMode::Fixed(50_000_000_000u128),
            )
            .unwrap();
        assert_eq!(trigger_id, 1);
        let trigger = contract.get_claim_trigger(trigger_id).unwrap();
        assert_eq!(trigger.policy_id, policy_id);
        assert!(trigger.is_active);
        assert!(!trigger.triggered);
        assert_eq!(contract.get_policy_triggers(policy_id), [trigger_id]);
    }

    #[ink::test]
    fn test_register_trigger_unauthorized() {
        let (mut contract, policy_id, _) = setup_with_active_policy();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let result = contract.register_claim_trigger(
            policy_id,
            TriggerMetric::WindSpeed,
            TriggerComparator::GreaterOrEqual,
            200,
            PayoutMode::Fixed(1u128),
        );
        assert_eq!(result, Err(InsuranceError::Unauthorized));
    }

    #[ink::test]
    fn test_register_trigger_invalid_payout_mode() {
        let (mut contract, policy_id, _) = setup_with_active_policy();
        let result = contract.register_claim_trigger(
            policy_id,
            TriggerMetric::WindSpeed,
            TriggerComparator::GreaterOrEqual,
            200,
            PayoutMode::Fixed(0u128),
        );
        assert_eq!(result, Err(InsuranceError::InvalidPayoutMode));

        let result = contract.register_claim_trigger(
            policy_id,
            TriggerMetric::WindSpeed,
            TriggerComparator::GreaterOrEqual,
            200,
            PayoutMode::PercentBps(20_000),
        );
        assert_eq!(result, Err(InsuranceError::InvalidPayoutMode));
    }

    #[ink::test]
    fn test_oracle_event_below_threshold_does_not_pay() {
        let (mut contract, policy_id, _) = setup_with_active_policy();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let trigger_id = contract
            .register_claim_trigger(
                policy_id,
                TriggerMetric::WindSpeed,
                TriggerComparator::GreaterOrEqual,
                200,
                PayoutMode::Fixed(50_000_000_000u128),
            )
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.authorize_oracle(accounts.charlie).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);

        let result = contract.report_oracle_event(trigger_id, 150, "ipfs://r".into());
        assert_eq!(result, Ok(None));
        let trigger = contract.get_claim_trigger(trigger_id).unwrap();
        assert!(!trigger.triggered);
        assert_eq!(trigger.last_observed_value, Some(150));
        assert_eq!(contract.get_claim_count(), 0);
    }

    #[ink::test]
    fn test_oracle_event_meets_threshold_auto_pays() {
        let (mut contract, policy_id, _coverage) = setup_with_active_policy();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let trigger_id = contract
            .register_claim_trigger(
                policy_id,
                TriggerMetric::WindSpeed,
                TriggerComparator::GreaterOrEqual,
                200,
                PayoutMode::Fixed(50_000_000_000u128),
            )
            .unwrap();

        // Authorized oracle reports an event meeting threshold.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.authorize_oracle(accounts.charlie).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);

        let result = contract.report_oracle_event(trigger_id, 250, "ipfs://hurricane".into());
        let claim_id = result.unwrap().expect("expected an auto-claim id");

        let trigger = contract.get_claim_trigger(trigger_id).unwrap();
        assert!(trigger.triggered);
        assert_eq!(trigger.triggering_claim_id, Some(claim_id));
        let claim = contract.get_claim(claim_id).unwrap();
        assert_eq!(claim.status, ClaimStatus::Paid);
        assert_eq!(claim.policy_id, policy_id);
        assert_eq!(claim.claimant, accounts.bob);
        assert!(claim.payout_amount > 0);
        assert_eq!(claim.oracle_report_url, "ipfs://hurricane");

        let policy = contract.get_policy(policy_id).unwrap();
        assert_eq!(policy.claims_count, 1);
        assert!(policy.total_claimed > 0);
    }

    #[ink::test]
    fn test_oracle_event_less_or_equal_comparator() {
        let (mut contract, policy_id, _) = setup_with_active_policy();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let trigger_id = contract
            .register_claim_trigger(
                policy_id,
                TriggerMetric::Temperature,
                TriggerComparator::LessOrEqual,
                10, // freezing-point trigger
                PayoutMode::Fixed(20_000_000_000u128),
            )
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.authorize_oracle(accounts.charlie).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        // value <= threshold should fire
        let claim_id = contract
            .report_oracle_event(trigger_id, 5, "ipfs://cold".into())
            .unwrap();
        assert!(claim_id.is_some());
        assert!(contract.get_claim_trigger(trigger_id).unwrap().triggered);
    }

    #[ink::test]
    fn test_oracle_event_unauthorized_caller() {
        let (mut contract, policy_id, _) = setup_with_active_policy();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let trigger_id = contract
            .register_claim_trigger(
                policy_id,
                TriggerMetric::WindSpeed,
                TriggerComparator::GreaterOrEqual,
                200,
                PayoutMode::Fixed(50_000_000_000u128),
            )
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.django);
        let result = contract.report_oracle_event(trigger_id, 250, "ipfs://r".into());
        assert_eq!(result, Err(InsuranceError::Unauthorized));
    }

    #[ink::test]
    fn test_oracle_event_trigger_already_fired() {
        let (mut contract, policy_id, _) = setup_with_active_policy();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let trigger_id = contract
            .register_claim_trigger(
                policy_id,
                TriggerMetric::WindSpeed,
                TriggerComparator::GreaterOrEqual,
                200,
                PayoutMode::Fixed(50_000_000_000u128),
            )
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.authorize_oracle(accounts.charlie).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        contract
            .report_oracle_event(trigger_id, 250, "ipfs://r1".into())
            .unwrap();
        let result = contract.report_oracle_event(trigger_id, 300, "ipfs://r2".into());
        assert_eq!(result, Err(InsuranceError::TriggerAlreadyFired));
    }

    #[ink::test]
    fn test_deactivated_trigger_does_not_fire() {
        let (mut contract, policy_id, _) = setup_with_active_policy();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let trigger_id = contract
            .register_claim_trigger(
                policy_id,
                TriggerMetric::WindSpeed,
                TriggerComparator::GreaterOrEqual,
                200,
                PayoutMode::Fixed(50_000_000_000u128),
            )
            .unwrap();
        // Bob (policyholder) deactivates.
        contract.deactivate_claim_trigger(trigger_id).unwrap();
        let trigger = contract.get_claim_trigger(trigger_id).unwrap();
        assert!(!trigger.is_active);

        // Oracle reports — should reject as inactive.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.authorize_oracle(accounts.charlie).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let result = contract.report_oracle_event(trigger_id, 250, "ipfs://r".into());
        assert_eq!(result, Err(InsuranceError::TriggerInactive));
    }

    #[ink::test]
    fn test_deactivate_already_inactive_fails() {
        let (mut contract, policy_id, _) = setup_with_active_policy();
        let trigger_id = contract
            .register_claim_trigger(
                policy_id,
                TriggerMetric::WindSpeed,
                TriggerComparator::GreaterOrEqual,
                200,
                PayoutMode::Fixed(50_000_000_000u128),
            )
            .unwrap();
        contract.deactivate_claim_trigger(trigger_id).unwrap();
        assert_eq!(
            contract.deactivate_claim_trigger(trigger_id),
            Err(InsuranceError::TriggerInactive),
        );
    }

    #[ink::test]
    fn test_full_coverage_payout_mode() {
        let (mut contract, policy_id, coverage) = setup_with_active_policy();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let trigger_id = contract
            .register_claim_trigger(
                policy_id,
                TriggerMetric::EarthquakeMagnitude,
                TriggerComparator::GreaterOrEqual,
                700, // magnitude 7.0
                PayoutMode::FullCoverage,
            )
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.authorize_oracle(accounts.charlie).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);

        let claim_id = contract
            .report_oracle_event(trigger_id, 750, "ipfs://quake".into())
            .unwrap()
            .unwrap();
        let claim = contract.get_claim(claim_id).unwrap();
        // Gross claim equals full coverage; payout deducts the deductible.
        let policy = contract.get_policy(policy_id).unwrap();
        assert_eq!(claim.claim_amount, coverage);
        assert_eq!(claim.payout_amount, coverage.saturating_sub(policy.deductible));
        // total_claimed tracks net payout, so only `deductible` worth of
        // coverage remains available — policy stays Active.
        assert_eq!(policy.total_claimed, claim.payout_amount);
    }

    #[ink::test]
    fn test_percent_bps_payout_mode() {
        let (mut contract, policy_id, coverage) = setup_with_active_policy();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let trigger_id = contract
            .register_claim_trigger(
                policy_id,
                TriggerMetric::FloodLevel,
                TriggerComparator::GreaterOrEqual,
                100,
                PayoutMode::PercentBps(2_500), // 25% of remaining coverage
            )
            .unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.authorize_oracle(accounts.charlie).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);

        let claim_id = contract
            .report_oracle_event(trigger_id, 150, "ipfs://flood".into())
            .unwrap()
            .unwrap();
        let claim = contract.get_claim(claim_id).unwrap();
        assert_eq!(claim.claim_amount, coverage / 4);
    }

    #[ink::test]
    fn test_oracle_event_for_inactive_policy_fails() {
        let (mut contract, policy_id, _) = setup_with_active_policy();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let trigger_id = contract
            .register_claim_trigger(
                policy_id,
                TriggerMetric::WindSpeed,
                TriggerComparator::GreaterOrEqual,
                200,
                PayoutMode::Fixed(50_000_000_000u128),
            )
            .unwrap();

        // Cancel the policy.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.cancel_policy(policy_id).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.authorize_oracle(accounts.charlie).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let result = contract.report_oracle_event(trigger_id, 250, "ipfs://r".into());
        assert_eq!(result, Err(InsuranceError::PolicyInactive));
    }

    #[ink::test]
    fn test_admin_can_act_as_oracle() {
        let (mut contract, policy_id, _) = setup_with_active_policy();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let trigger_id = contract
            .register_claim_trigger(
                policy_id,
                TriggerMetric::Generic,
                TriggerComparator::GreaterOrEqual,
                100,
                PayoutMode::Fixed(10_000_000_000u128),
            )
            .unwrap();
        // Admin reports without going through authorize_oracle.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let claim_id = contract
            .report_oracle_event(trigger_id, 200, "ipfs://generic".into())
            .unwrap()
            .unwrap();
        assert!(contract.get_claim(claim_id).is_some());
    }
}
