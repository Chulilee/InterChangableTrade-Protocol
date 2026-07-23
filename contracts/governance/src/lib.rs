#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    NextProposalId,
    Proposal(u64),
    ProposalVotes(u64),
    ProposalQueue(u64),
    ProposalState(u64),
    ProposalMetadata(u64),
    Parameter(Symbol),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalState {
    Draft = 1,
    Active = 2,
    Succeeded = 3,
    Failed = 4,
    Executed = 5,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalMetadata {
    pub proposer: Address,
    pub description: Symbol,
    pub target: Symbol,
    pub value: i128,
    pub start_time: u64,
    pub end_time: u64,
    pub timelock: u64,
    pub quorum: u128,
    pub threshold: u128,
    pub canceled: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoteRecord {
    pub voter: Address,
    pub weight: u128,
    pub support: bool,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    ProposalNotFound = 4,
    InvalidState = 5,
    VotingClosed = 6,
    TimelockNotElapsed = 7,
    AlreadyQueued = 8,
    AlreadyExecuted = 9,
    Cancelled = 10,
    InvalidThreshold = 11,
    InvalidQuorum = 12,
}

const EVT_PROPOSAL_CREATED: Symbol = symbol_short!("prop_crtd");
const EVT_VOTE_CAST: Symbol = symbol_short!("vote_cast");
const EVT_PROPOSAL_QUEUED: Symbol = symbol_short!("prop_qd");
const EVT_PROPOSAL_EXECUTED: Symbol = symbol_short!("prop_exec");

#[contract]
pub struct Governance;

#[contractimpl]
impl Governance {
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextProposalId, &0u64);
        Ok(())
    }

    pub fn propose(
        env: Env,
        proposer: Address,
        description: Symbol,
        target: Symbol,
        value: i128,
        start_time: u64,
        end_time: u64,
        timelock: u64,
        quorum: u128,
        threshold: u128,
    ) -> Result<u64, Error> {
        proposer.require_auth();
        if quorum == 0 {
            return Err(Error::InvalidQuorum);
        }
        if threshold == 0 {
            return Err(Error::InvalidThreshold);
        }

        let proposal_id = Self::next_proposal_id(&env);
        let metadata = ProposalMetadata {
            proposer: proposer.clone(),
            description: description.clone(),
            target: target.clone(),
            value,
            start_time,
            end_time,
            timelock,
            quorum,
            threshold,
            canceled: false,
        };
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &metadata);
        env.storage().persistent().set(&DataKey::ProposalState(proposal_id), &ProposalState::Active);
        env.storage().persistent().set(&DataKey::ProposalVotes(proposal_id), &Vec::<VoteRecord>::new(&env));
        env.storage().persistent().set(&DataKey::ProposalQueue(proposal_id), &false);
        env.storage().persistent().set(&DataKey::ProposalMetadata(proposal_id), &metadata);

        env.events().publish(
            (Symbol::from_str(&env, "ProposalCreated"), proposal_id, proposer, description, target, value),
            (start_time, end_time, timelock, quorum, threshold),
        );
        Ok(proposal_id)
    }

    pub fn cast_vote(env: Env, proposal_id: u64, voter: Address, support: bool, weight: u128) -> Result<(), Error> {
        voter.require_auth();
        let mut metadata = Self::get_metadata(&env, proposal_id)?;
        if metadata.canceled {
            return Err(Error::Cancelled);
        }
        let state = Self::read_state(&env, proposal_id)?;
        if state != ProposalState::Active {
            return Err(Error::InvalidState);
        }
        let now = env.ledger().timestamp();
        if now < metadata.start_time || now > metadata.end_time {
            return Err(Error::VotingClosed);
        }

        let votes_key = DataKey::ProposalVotes(proposal_id);
        let mut votes: Vec<VoteRecord> = env.storage().persistent().get(&votes_key).unwrap_or(Vec::new(&env));
        let existing = Self::find_vote(&env, &votes, voter.clone());
        if let Some(index) = existing {
            let record = votes.get(index).unwrap();
            votes.remove(index as u32);
            if record.support != support || record.weight != weight {
                votes.push_back(VoteRecord { voter: voter.clone(), weight, support });
            }
        } else {
            votes.push_back(VoteRecord { voter: voter.clone(), weight, support });
        }
        env.storage().persistent().set(&votes_key, &votes);
        env.events().publish((Symbol::from_str(&env, "VoteCast"), proposal_id, voter, support), weight);
        Ok(())
    }

    pub fn queue_execution(env: Env, proposal_id: u64, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        let mut metadata = Self::get_metadata(&env, proposal_id)?;
        if metadata.canceled {
            return Err(Error::Cancelled);
        }
        let state = Self::read_state(&env, proposal_id)?;
        if state != ProposalState::Succeeded {
            return Err(Error::InvalidState);
        }
        let queued = env.storage().persistent().get(&DataKey::ProposalQueue(proposal_id)).unwrap_or(false);
        if queued {
            return Err(Error::AlreadyQueued);
        }
        let now = env.ledger().timestamp();
        if now < metadata.end_time + metadata.timelock {
            return Err(Error::TimelockNotElapsed);
        }
        env.storage().persistent().set(&DataKey::ProposalQueue(proposal_id), &true);
        env.events().publish((Symbol::from_str(&env, "ProposalQueued"), proposal_id, caller), (now, metadata.timelock));
        Ok(())
    }

    pub fn execute_proposal(env: Env, proposal_id: u64, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        let mut metadata = Self::get_metadata(&env, proposal_id)?;
        if metadata.canceled {
            return Err(Error::Cancelled);
        }
        let state = Self::read_state(&env, proposal_id)?;
        if state != ProposalState::Succeeded {
            return Err(Error::InvalidState);
        }
        let queued = env.storage().persistent().get(&DataKey::ProposalQueue(proposal_id)).unwrap_or(false);
        if !queued {
            return Err(Error::AlreadyQueued);
        }
        let now = env.ledger().timestamp();
        if now < metadata.end_time + metadata.timelock {
            return Err(Error::TimelockNotElapsed);
        }
        Self::set_state(&env, proposal_id, ProposalState::Executed);
        env.storage()
            .persistent()
            .set(&DataKey::Parameter(metadata.target.clone()), &metadata.value);
        env.events().publish((Symbol::from_str(&env, "ProposalExecuted"), proposal_id, caller), metadata.value);
        Ok(())
    }

    pub fn cancel_proposal(env: Env, proposal_id: u64, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        let mut metadata = Self::get_metadata(&env, proposal_id)?;
        if metadata.canceled {
            return Err(Error::Cancelled);
        }
        let state = Self::read_state(&env, proposal_id)?;
        if state == ProposalState::Executed {
            return Err(Error::AlreadyExecuted);
        }
        metadata.canceled = true;
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &metadata);
        env.storage().persistent().set(&DataKey::ProposalMetadata(proposal_id), &metadata);
        Ok(())
    }

    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<ProposalMetadata, Error> {
        Self::get_metadata(&env, proposal_id)
    }

    pub fn get_state(env: Env, proposal_id: u64) -> Result<ProposalState, Error> {
        Self::read_state(&env, proposal_id)
    }

    pub fn get_parameter(env: Env, key: Symbol) -> Option<i128> {
        env.storage().persistent().get(&DataKey::Parameter(key))
    }

    pub fn get_votes(env: Env, proposal_id: u64) -> Result<Vec<VoteRecord>, Error> {
        Self::get_votes_internal(&env, proposal_id)
    }

    pub fn finalize(env: Env, proposal_id: u64) -> Result<(), Error> {
        let metadata = Self::get_metadata(&env, proposal_id)?;
        if metadata.canceled {
            return Err(Error::Cancelled);
        }
        let state = Self::read_state(&env, proposal_id)?;
        if state != ProposalState::Active {
            return Err(Error::InvalidState);
        }
        let now = env.ledger().timestamp();
        if now <= metadata.end_time {
            return Err(Error::VotingClosed);
        }
        let votes = Self::get_votes_internal(&env, proposal_id)?;
        let mut yes_weight = 0u128;
        let mut no_weight = 0u128;
        for i in 0..votes.len() {
            let record = votes.get(i as u32).unwrap();
            if record.support {
                yes_weight += record.weight;
            } else {
                no_weight += record.weight;
            }
        }
        let next_state = if yes_weight >= metadata.quorum && yes_weight > no_weight && yes_weight >= metadata.threshold {
            ProposalState::Succeeded
        } else {
            ProposalState::Failed
        };
        Self::set_state(&env, proposal_id, next_state.clone());
        Ok(())
    }

    fn next_proposal_id(env: &Env) -> u64 {
        let mut next_id = env.storage().instance().get(&DataKey::NextProposalId).unwrap_or(0u64);
        next_id += 1;
        env.storage().instance().set(&DataKey::NextProposalId, &next_id);
        next_id
    }

    fn get_metadata(env: &Env, proposal_id: u64) -> Result<ProposalMetadata, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)
    }

    fn read_state(env: &Env, proposal_id: u64) -> Result<ProposalState, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::ProposalState(proposal_id))
            .ok_or(Error::ProposalNotFound)
    }

    fn get_votes_internal(env: &Env, proposal_id: u64) -> Result<Vec<VoteRecord>, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::ProposalVotes(proposal_id))
            .ok_or(Error::ProposalNotFound)
    }

    fn find_vote(env: &Env, votes: &Vec<VoteRecord>, voter: Address) -> Option<u32> {
        let mut idx = 0u32;
        for i in 0..votes.len() {
            let record = votes.get(i as u32).unwrap();
            if record.voter == voter {
                return Some(idx);
            }
            idx += 1;
        }
        None
    }

    fn set_state(env: &Env, proposal_id: u64, state: ProposalState) {
        env.storage().persistent().set(&DataKey::ProposalState(proposal_id), &state);
    }
}

#[cfg(test)]
mod test;
