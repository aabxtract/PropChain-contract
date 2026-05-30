// Unit tests for the staking contract (Issue #101 - extracted from lib.rs)

#[cfg(test)]
mod tests {
    use super::*;

    fn default_accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
    }

    fn set_caller(caller: AccountId) {
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(caller);
    }

    fn advance_block(n: u32) {
        for _ in 0..n {
            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
        }
    }

    fn create_staking() -> Staking {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        Staking::new(500, 1_000)
    }

    #[ink::test]
    fn constructor_sets_defaults() {
        let staking = create_staking();
        let accounts = default_accounts();
        assert_eq!(staking.get_admin(), accounts.alice);
        assert_eq!(staking.get_total_staked(), 0);
        assert_eq!(staking.get_reward_pool(), 0);
        assert_eq!(staking.get_min_stake(), 1_000);
    }

    #[ink::test]
    fn constructor_clamps_zero_min_stake() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let staking = Staking::new(500, 0);
        assert_eq!(staking.get_min_stake(), constants::STAKING_MIN_AMOUNT);
    }

    #[ink::test]
    fn stake_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        let result = staking.stake(10_000, LockPeriod::Flexible);
        assert!(result.is_ok());
        assert_eq!(staking.get_total_staked(), 10_000);

        let info = staking.get_stake(accounts.bob).unwrap();
        assert_eq!(info.amount, 10_000);
        assert_eq!(info.lock_period, LockPeriod::Flexible);
    }

    #[ink::test]
    fn stake_below_minimum_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(
            staking.stake(500, LockPeriod::Flexible),
            Err(Error::InsufficientAmount)
        );
    }

    #[ink::test]
    fn stake_zero_amount_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(
            staking.stake(0, LockPeriod::Flexible),
            Err(Error::ZeroAmount)
        );
    }

    #[ink::test]
    fn double_stake_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        assert_eq!(
            staking.stake(10_000, LockPeriod::Flexible),
            Err(Error::AlreadyStaked)
        );
    }

    #[ink::test]
    fn unstake_flexible_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let result = staking.unstake();
        assert!(result.is_ok());
        assert_eq!(staking.get_total_staked(), 0);
        assert!(staking.get_stake(accounts.bob).is_none());
    }

    #[ink::test]
    fn unstake_locked_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::ThirtyDays).unwrap();
        assert_eq!(staking.unstake(), Err(Error::LockActive));
    }

    #[ink::test]
    fn unstake_no_stake_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(staking.unstake(), Err(Error::StakeNotFound));
    }

    #[ink::test]
    fn claim_rewards_with_pool() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(10_000_000_000_000).unwrap();

        set_caller(accounts.bob);
        staking
            .stake(1_000_000_000_000_000, LockPeriod::Flexible)
            .unwrap();

        advance_block(100_000);

        let pending = staking.get_pending_rewards(accounts.bob);
        assert!(
            pending > 0,
            "pending rewards should be > 0, got {}",
            pending
        );

        let result = staking.claim_rewards();
        assert!(result.is_ok());
    }

    #[ink::test]
    fn claim_rewards_no_stake_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(staking.claim_rewards(), Err(Error::StakeNotFound));
    }

    #[ink::test]
    fn delegate_governance_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();

        assert_eq!(staking.get_governance_power(accounts.bob), 10_000);

        staking.delegate_governance(accounts.charlie).unwrap();
        assert_eq!(staking.get_governance_power(accounts.bob), 0);
        assert_eq!(staking.get_governance_power(accounts.charlie), 10_000);
    }

    #[ink::test]
    fn self_delegation_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        assert_eq!(
            staking.delegate_governance(accounts.bob),
            Err(Error::InvalidDelegate)
        );
    }

    #[ink::test]
    fn fund_pool_non_admin_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(staking.fund_reward_pool(1000), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn update_config_succeeds() {
        let mut staking = create_staking();
        staking.update_config(5_000, 1000).unwrap();
        assert_eq!(staking.get_min_stake(), 5_000);
    }

    #[ink::test]
    fn update_config_zero_min_fails() {
        let mut staking = create_staking();
        assert_eq!(staking.update_config(0, 1000), Err(Error::InvalidConfig));
    }

    #[ink::test]
    fn lock_period_durations_correct() {
        assert_eq!(LockPeriod::Flexible.duration_blocks(), 0);
        assert_eq!(
            LockPeriod::ThirtyDays.duration_blocks(),
            constants::LOCK_PERIOD_30_DAYS
        );
        assert_eq!(
            LockPeriod::NinetyDays.duration_blocks(),
            constants::LOCK_PERIOD_90_DAYS
        );
        assert_eq!(
            LockPeriod::OneYear.duration_blocks(),
            constants::LOCK_PERIOD_1_YEAR
        );
    }

    #[ink::test]
    fn multipliers_increase_with_lock() {
        assert!(LockPeriod::ThirtyDays.multiplier() > LockPeriod::Flexible.multiplier());
        assert!(LockPeriod::NinetyDays.multiplier() > LockPeriod::ThirtyDays.multiplier());
        assert!(LockPeriod::OneYear.multiplier() > LockPeriod::NinetyDays.multiplier());
    }

    // ----- Parameter governance -----

    fn end_voting_period(staking: &Staking) {
        let (period, _) = staking.get_voting_config();
        advance_block(period as u32 + 1);
    }

    #[ink::test]
    fn propose_requires_voting_power() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(
            staking.propose_param_change(ParamKind::MinStake(2_000)),
            Err(Error::NoVotingPower),
        );
    }

    #[ink::test]
    fn propose_param_change_records_proposal() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();

        let id = staking
            .propose_param_change(ParamKind::MinStake(2_000))
            .unwrap();
        assert_eq!(id, 0);
        let p = staking.get_param_proposal(0).unwrap();
        assert_eq!(p.kind, ParamKind::MinStake(2_000));
        assert_eq!(p.status, ProposalStatus::Active);
        assert_eq!(p.total_power_snapshot, 10_000);
    }

    #[ink::test]
    fn propose_invalid_param_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        assert_eq!(
            staking.propose_param_change(ParamKind::MinStake(0)),
            Err(Error::InvalidConfig),
        );
        assert_eq!(
            staking.propose_param_change(ParamKind::QuorumBps(20_000)),
            Err(Error::InvalidConfig),
        );
    }

    #[ink::test]
    fn vote_weight_uses_governance_power() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();

        let id = staking
            .propose_param_change(ParamKind::RewardRateBps(750))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();

        let p = staking.get_param_proposal(id).unwrap();
        assert_eq!(p.votes_for, 10_000);
        assert_eq!(p.votes_against, 0);
        assert!(staking.has_voted(id, accounts.bob));
    }

    #[ink::test]
    fn double_vote_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::RewardRateBps(750))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();
        assert_eq!(staking.vote_on_proposal(id, false), Err(Error::AlreadyVoted));
    }

    #[ink::test]
    fn vote_without_power_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::RewardRateBps(750))
            .unwrap();

        set_caller(accounts.charlie);
        assert_eq!(staking.vote_on_proposal(id, true), Err(Error::NoVotingPower));
    }

    #[ink::test]
    fn execute_applies_winning_proposal() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();

        end_voting_period(&staking);
        staking.execute_param_proposal(id).unwrap();

        assert_eq!(staking.get_min_stake(), 2_500);
        let p = staking.get_param_proposal(id).unwrap();
        assert_eq!(p.status, ProposalStatus::Executed);
    }

    #[ink::test]
    fn execute_before_voting_end_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();
        assert_eq!(
            staking.execute_param_proposal(id),
            Err(Error::VotingActive),
        );
    }

    #[ink::test]
    fn vote_after_voting_end_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        end_voting_period(&staking);
        assert_eq!(staking.vote_on_proposal(id, true), Err(Error::VotingEnded));
    }

    #[ink::test]
    fn execute_quorum_not_reached_rejects() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        // Two stakers; only one of them votes. Quorum is 10% of total stake
        // so a single voter with > 10% of the supply still meets quorum —
        // pick weights so quorum is missed instead.
        set_caller(accounts.bob);
        staking.stake(1_000, LockPeriod::Flexible).unwrap();
        set_caller(accounts.charlie);
        staking.stake(1_000_000, LockPeriod::Flexible).unwrap();

        set_caller(accounts.bob);
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();

        end_voting_period(&staking);
        assert_eq!(
            staking.execute_param_proposal(id),
            Err(Error::QuorumNotReached),
        );

        let p = staking.get_param_proposal(id).unwrap();
        assert_eq!(p.status, ProposalStatus::Rejected);
        // Original min_stake unchanged.
        assert_eq!(staking.get_min_stake(), 1_000);
    }

    #[ink::test]
    fn execute_majority_against_rejects() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        set_caller(accounts.charlie);
        staking.stake(20_000, LockPeriod::Flexible).unwrap();

        set_caller(accounts.bob);
        let id = staking
            .propose_param_change(ParamKind::MinStake(5_000))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();

        set_caller(accounts.charlie);
        staking.vote_on_proposal(id, false).unwrap();

        end_voting_period(&staking);
        // No quorum failure: Ok(()) but proposal rejected and parameter unchanged.
        staking.execute_param_proposal(id).unwrap();
        let p = staking.get_param_proposal(id).unwrap();
        assert_eq!(p.status, ProposalStatus::Rejected);
        assert_eq!(staking.get_min_stake(), 1_000);
    }

    #[ink::test]
    fn execute_twice_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();
        end_voting_period(&staking);
        staking.execute_param_proposal(id).unwrap();
        assert_eq!(
            staking.execute_param_proposal(id),
            Err(Error::ProposalClosed),
        );
    }

    #[ink::test]
    fn cancel_by_proposer_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        staking.cancel_param_proposal(id).unwrap();
        let p = staking.get_param_proposal(id).unwrap();
        assert_eq!(p.status, ProposalStatus::Cancelled);
    }

    #[ink::test]
    fn cancel_by_outsider_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        set_caller(accounts.charlie);
        assert_eq!(
            staking.cancel_param_proposal(id),
            Err(Error::Unauthorized),
        );
    }

    #[ink::test]
    fn voting_period_can_be_governed() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();

        let id = staking
            .propose_param_change(ParamKind::VotingPeriodBlocks(100))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();
        end_voting_period(&staking);
        staking.execute_param_proposal(id).unwrap();

        assert_eq!(staking.get_voting_config().0, 100);
    }

    #[ink::test]
    fn delegate_can_vote_with_full_power() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        staking.delegate_governance(accounts.charlie).unwrap();

        // Bob has no power any more.
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_000))
            .ok();
        assert!(id.is_none());

        // Charlie now holds Bob's power and can drive the proposal.
        set_caller(accounts.charlie);
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_000))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();
        end_voting_period(&staking);
        staking.execute_param_proposal(id).unwrap();

        assert_eq!(staking.get_min_stake(), 2_000);
    }

    #[ink::test]
    fn set_auto_compound_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        
        let stake_info = staking.get_stake(accounts.bob).unwrap();
        assert_eq!(stake_info.auto_compound, false);
        
        staking.set_auto_compound(true).unwrap();
        let stake_info = staking.get_stake(accounts.bob).unwrap();
        assert_eq!(stake_info.auto_compound, true);
    }

    #[ink::test]
    fn auto_compounding_reinvests_rewards() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        
        set_caller(accounts.alice);
        staking.fund_reward_pool(10_000_000_000_000).unwrap();

        set_caller(accounts.bob);
        staking.stake(1_000_000_000_000_000, LockPeriod::Flexible).unwrap();
        staking.set_auto_compound(true).unwrap();

        advance_block(100_000);

        let initial_stake = staking.get_stake(accounts.bob).unwrap().amount;
        let pending = staking.get_pending_rewards(accounts.bob);
        assert!(pending > 0);

        staking.claim_rewards().unwrap();

        let final_stake = staking.get_stake(accounts.bob).unwrap().amount;
        assert_eq!(final_stake, initial_stake + pending);
    }

    #[ink::test]
  
#[ink::test]
fn early_withdrawal_applies_penalty() {
    let mut staking = create_staking();
    let accounts = default_accounts();
    set_caller(accounts.bob);

    staking.stake(10_000_000_000_000, LockPeriod::ThirtyDays).unwrap();

    // Unstake immediately — lock not expired, penalty should apply
    assert!(staking.unstake().is_ok());

    assert_eq!(staking.get_total_staked(), 0);
    // 10% of 10_000_000_000_000 = 1_000_000_000_000 went to reward pool
    assert!(staking.get_reward_pool() >= 1_000_000_000_000);
}

#[ink::test]
fn flexible_lock_no_penalty_on_early_unstake() {
    let mut staking = create_staking();
    let accounts = default_accounts();
    set_caller(accounts.bob);

    staking.stake(10_000_000_000_000, LockPeriod::Flexible).unwrap();

    let pool_before = staking.get_reward_pool();
    assert!(staking.unstake().is_ok());
    assert_eq!(staking.get_total_staked(), 0);
    // No penalty for flexible
    assert_eq!(staking.get_reward_pool(), pool_before);
}

#[ink::test]
fn no_penalty_after_lock_expires() {
    let mut staking = create_staking();
    let accounts = default_accounts();
    set_caller(accounts.bob);

    staking.stake(10_000_000_000_000, LockPeriod::ThirtyDays).unwrap();

    // Advance past the 30-day lock period
    advance_block(constants::LOCK_PERIOD_30_DAYS as u32 + 1);

    let pool_before = staking.get_reward_pool();
    assert!(staking.unstake().is_ok());
    // Reward pool unchanged — no penalty after expiry
    assert_eq!(staking.get_reward_pool(), pool_before);
}

#[ink::test]
fn set_early_withdrawal_penalty_admin_only() {
    let mut staking = create_staking();
    let accounts = default_accounts();

    // Admin (alice) can update
    assert!(staking.set_early_withdrawal_penalty(500).is_ok());
    assert_eq!(staking.get_early_withdrawal_penalty_bps(), 500);

    // Non-admin cannot
    set_caller(accounts.bob);
    assert_eq!(
        staking.set_early_withdrawal_penalty(200),
        Err(Error::Unauthorized)
    );
}

#[ink::test]
fn set_early_withdrawal_penalty_max_cap() {
    let mut staking = create_staking();

    // Above 50% cap is rejected
    assert_eq!(
        staking.set_early_withdrawal_penalty(6_000),
        Err(Error::InvalidConfig)
    );

    // Exactly at cap is fine
    assert!(staking.set_early_withdrawal_penalty(5_000).is_ok());
}
    fn staking_tiers_applied_correctly() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        
        // Bob stakes Bronze amount (< 10_000)
        set_caller(accounts.bob);
        staking.stake(5_000, LockPeriod::Flexible).unwrap();
        assert_eq!(staking.get_staker_tier(accounts.bob), StakingTier::Bronze);

        // Charlie stakes Silver amount (>= 10_000)
        set_caller(accounts.charlie);
        staking.stake(15_000, LockPeriod::Flexible).unwrap();
        assert_eq!(staking.get_staker_tier(accounts.charlie), StakingTier::Silver);

        // Django stakes Gold amount (>= 50_000)
        let django = accounts.django;
        set_caller(django);
        staking.stake(55_000, LockPeriod::Flexible).unwrap();
        assert_eq!(staking.get_staker_tier(django), StakingTier::Gold);

        // Verify tier name and multiplier
        assert_eq!(StakingTier::Bronze.name(), "Bronze");
        assert_eq!(StakingTier::Bronze.reward_multiplier(), 100);
        assert_eq!(StakingTier::Silver.reward_multiplier(), 110);
        assert_eq!(StakingTier::Gold.reward_multiplier(), 120);
        assert_eq!(StakingTier::Platinum.reward_multiplier(), 135);
        assert_eq!(StakingTier::Diamond.reward_multiplier(), 150);
    }
}

