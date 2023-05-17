use affair::{AsyncWorker, Executor, Port, TokioSpawn, Worker as WorkerTrait};
use anyhow::Result;
use fastcrypto::traits::ToFromBytes;
use fastcrypto::{bls12381, ed25519};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

pub type CommodityPrice = u128;
pub type ServiceId = u64;
pub type Epoch = u64;
pub type BLSPublicKey = bls12381::min_sig::BLS12381PublicKey;
pub type PublicKey = ed25519::Ed25519PublicKey;
pub type NetworkKey = ed25519::Ed25519PublicKey;
pub type ApplicationQuery = Port<Transaction, TransactionResponse>;
pub type ApplicationUpdate = Port<Transaction, TransactionResponse>;

pub struct App {
    /// Cloneable port to send txns to the application layer
    update_socket: ApplicationUpdate,
    /// Cloneable port used to send querys to the application layer
    query_socket: ApplicationQuery,
}

impl App {
    /// Creates and runs the application
    pub fn new<E: Env>(mut env: E) -> Self {
        env.genesis();
        Self {
            query_socket: TokioSpawn::spawn_async(QueryWorker::new(env.query())),
            update_socket: TokioSpawn::spawn(UpdateWorker::new(env)),
        }
    }
    /// Get the port for sending Query transactions to the application
    pub fn get_query_socket(&self) -> ApplicationQuery {
        self.query_socket.clone()
    }
    /// Get the port for sending update to the application layer. This should only be called once and given to Narwhal
    pub fn get_update_socket(&self) -> ApplicationUpdate {
        self.update_socket.clone()
    }
}

pub trait Env: Send + 'static {
    type Query: QueryEnv + 'static;
    /// This function should create an instance of ApplicationState and submit the transaction to ApplicationState::execute_txn
    fn run(&mut self, transaction: Transaction) -> TransactionResponse;
    /// Returns the query env
    fn query(&self) -> Self::Query;
    /// Load the genesis block for state. Is only called in the App::new()
    fn genesis(&mut self);
}

pub trait QueryEnv: Send {
    /// This should create an instance of ApplicationState submit the transaction, not committing the results
    fn run(&self, transaction: Transaction) -> TransactionResponse;
}

pub struct UpdateWorker<E: Env> {
    env: E,
}

impl<E: Env> UpdateWorker<E> {
    pub fn new(env: E) -> Self {
        Self { env }
    }
}

impl<E: Env + Send + 'static> WorkerTrait for UpdateWorker<E> {
    type Request = Transaction;
    type Response = TransactionResponse;
    fn handle(&mut self, req: Self::Request) -> Self::Response {
        // 1. Verify Signature and Nonce
        // Note(Dalton), this check will probably be moved to execute_txn function, and backend Arc will probably only be stored there as well
        // if let Err(err) = self.backend.verify_transaction(&req) {
        //     return TransactionResponse::Revert(err);
        // }
        // 2. Execute the transaction based on the transaction type

        self.env.run(req)
    }
}

pub struct QueryWorker<E: QueryEnv> {
    env: E,
}

impl<E: QueryEnv> QueryWorker<E> {
    pub fn new(env: E) -> Self {
        Self { env }
    }
}

#[async_trait::async_trait]
impl<E: QueryEnv + Send + 'static> AsyncWorker for QueryWorker<E> {
    type Request = Transaction;
    type Response = TransactionResponse;
    async fn handle(&mut self, req: Self::Request) -> Self::Response {
        // 1. Just execute transaction and return results, no need to do signature verification
        self.env.run(req)
    }
}

/// The state of the Application
///
/// The functions implemented on this struct are the "Smart Contracts" of the application layer
/// All state changes come from Transactions and start at execute_txn
pub struct ApplicationState<B: Backend> {
    pub metadata: B::Ref<Metadata, u64>,
    pub account_info: B::Ref<PublicKey, AccountInfo>,
    pub node_info: B::Ref<PublicKey, NodeInfo>,
    pub committee_info: B::Ref<Epoch, Committee>,
    pub bandwidth_info: B::Ref<Epoch, BandwidthInfo>,
    pub services: B::Ref<ServiceId, Service>,
    pub parameters: B::Ref<ProtocolParams, u128>,
    pub backend: B,
}

impl<B: Backend> ApplicationState<B> {
    pub fn new(backend: B) -> Self {
        Self {
            metadata: backend.get_table_reference("metadata"),
            account_info: backend.get_table_reference("account"),
            node_info: backend.get_table_reference("node"),
            committee_info: backend.get_table_reference("committee"),
            bandwidth_info: backend.get_table_reference("bandwidth"),
            services: backend.get_table_reference("service"),
            parameters: backend.get_table_reference("parameter"),
            backend,
        }
    }

    /// This function is the entry point of a transaction
    pub fn execute_txn(&self, txn: Transaction) -> TransactionResponse {
        match txn.transaction_type {
            TransactionType::ProofOfDelivery {
                client,
                commodity,
                service_id,
                proof,
            } => self.submit_pod(client, txn.sender, commodity, service_id, proof),

            TransactionType::Withdraw {
                amount,
                token,
                receiving_address,
            } => self.withdraw(txn.sender, receiving_address, amount, token),

            TransactionType::Deposit {
                proof,
                token,
                amount,
            } => self.deposit(txn.sender, proof, amount, token),

            TransactionType::Stake {
                proof,
                amount,
                node,
            } => self.stake(txn.sender, proof, amount, node),

            TransactionType::Unstake { amount } => self.unstake(txn.sender, amount),

            TransactionType::ChangeEpoch => self.change_epoch(txn.sender),

            TransactionType::AddService {
                service,
                service_id,
            } => self.add_service(txn.sender, service, service_id),

            TransactionType::RemoveService { service_id } => {
                self.remove_service(txn.sender, service_id)
            }

            TransactionType::Slash {
                service_id,
                node,
                proof_of_misbehavior,
            } => self.slash(txn.sender, proof_of_misbehavior, service_id, node),

            TransactionType::Query(query) => match query {
                Query::FLK { public_key } => self.get_flk(public_key),
                Query::Locked { public_key } => self.get_locked(public_key),
                Query::Bandwidth { public_key } => self.get_bandwidth(public_key),
                Query::Served { epoch, node } => self.get_node_bandwidth_served(epoch, node),
                Query::RewardPool { epoch } => self.get_reward_pool(epoch),
                Query::TotalServed { epoch } => self.get_total_served(epoch),
                Query::Staked { node } => self.get_staked(node),
                Query::CurrentEpochInfo => self.get_current_epoch_info(),
            },
        }
    }
    /*********** External Update Functions ***********/
    // The following functions should only be called in the result of a query or update transaction through execute_txn()
    // If called in an update txn it will mutate state
    fn submit_pod(
        &self,
        client: PublicKey,
        provider: PublicKey,
        commodity: u128,
        service_id: u64,
        proof: (),
    ) -> TransactionResponse {
        if !self.backend.verify_proof_of_delivery(
            &client,
            &provider,
            &commodity,
            &service_id,
            proof,
        ) {
            return TransactionResponse::Revert(ExecutionError::InvalidProof);
        }

        let current_epoch = self.metadata.get(&Metadata::Epoch).unwrap_or_default();
        let mut client_info = self.account_info.get(&client).unwrap_or_default();

        let cost = match self.services.get(&service_id) {
            Some(service) => {
                let real_cost = service.commodity_price * commodity;
                if client_info.bandwidth_balance < real_cost {
                    client_info.bandwidth_balance
                } else {
                    real_cost
                }
            }
            None => return TransactionResponse::Revert(ExecutionError::NonExistingService),
        };

        let mut bandwidth_info = self.bandwidth_info.get(&current_epoch).unwrap_or_default();

        bandwidth_info.total_served += commodity;

        bandwidth_info
            .bandwidth_per_node
            .entry(provider)
            .and_modify(|e| *e += commodity)
            .or_insert(commodity);

        bandwidth_info.reward_pool += cost;

        client_info.bandwidth_balance =
            client_info.bandwidth_balance.checked_sub(cost).unwrap_or(0);

        self.account_info.set(client, client_info);
        self.bandwidth_info.set(current_epoch, bandwidth_info);

        TransactionResponse::Success(ExecutionData::None)
    }

    fn withdraw(
        &self,
        sender: PublicKey,
        reciever: PublicKey,
        amount: u128,
        token: Tokens,
    ) -> TransactionResponse {
        todo!()
    }

    fn deposit(
        &self,
        sender: PublicKey,
        proof: ProofOfConsensus,
        amount: u128,
        token: Tokens,
    ) -> TransactionResponse {
        if !self.backend.verify_proof_of_consensus(proof) {
            return TransactionResponse::Revert(ExecutionError::InvalidProof);
        }

        let mut address = self.account_info.get(&sender).unwrap_or_default();

        match token {
            Tokens::FLK => address.flk_balance += amount,
            Tokens::USDC => address.bandwidth_balance += amount,
        }
        self.account_info.set(sender, address);
        TransactionResponse::Success(ExecutionData::None)
    }

    fn stake(
        &self,
        sender: PublicKey,
        proof: ProofOfConsensus,
        amount: u128,
        node: PublicKey,
    ) -> TransactionResponse {
        if !self.backend.verify_proof_of_consensus(proof) {
            return TransactionResponse::Revert(ExecutionError::InvalidProof);
        }
        // Check if node exists if it doesnt create it
        // Temporary:Will replace this flow before interface is finalized
        let node_info = match self.node_info.get(&node) {
            Some(node) => {
                if node.owner != sender {
                    return TransactionResponse::Revert(ExecutionError::NotNodeOwner);
                }
                node
            }
            None => return TransactionResponse::Revert(ExecutionError::NodeDoesNotExist),
        };

        let mut info = self.account_info.get(&sender).unwrap_or_default();

        info.staking.staked += amount;

        let new_balance = info.staking.staked;

        self.account_info.set(sender, info);
        self.node_info.set(node, node_info);

        TransactionResponse::Success(ExecutionData::UInt(new_balance))
    }

    fn unstake(&self, sender: PublicKey, amount: u128) -> TransactionResponse {
        todo!()
    }

    fn change_epoch(&self, sender: PublicKey) -> TransactionResponse {
        let mut current_epoch = self.metadata.get(&Metadata::Epoch).unwrap_or_default();
        let mut current_committee = self.committee_info.get(&current_epoch).unwrap_or_default();

        // If sender is not on the current committee revert early, or if they have already signaled;
        if !current_committee.members.contains(&sender) {
            return TransactionResponse::Revert(ExecutionError::NotCommitteeMember);
        } else if current_committee.ready_to_change.contains(&sender) {
            return TransactionResponse::Revert(ExecutionError::AlreadySignaled);
        }
        current_committee.ready_to_change.push(sender);

        // If more than 2/3rds of the committee have signaled, start the epoch change process
        if current_committee.ready_to_change.len() >= (current_committee.members.len() / 2) + 1 {
            // Todo: Reward nodes, calculate rep?, choose new committee, increment epoch.

            // calculate the next epoch endstamp
            let epoch_duration = self.parameters.get(&ProtocolParams::EpochTime).unwrap();
            let new_epoch_end = current_committee.epoch_end_timestamp + epoch_duration as u64;

            // Save the old committee so we can see who signaled
            self.committee_info.set(current_epoch, current_committee);
            // Get new committee
            let new_committee = self.choose_new_committee();
            // increment epoch
            current_epoch += 1;

            self.committee_info.set(
                current_epoch,
                Committee {
                    ready_to_change: Vec::with_capacity(new_committee.len()),
                    members: new_committee,
                    epoch_end_timestamp: new_epoch_end,
                },
            );
            self.metadata.set(Metadata::Epoch, current_epoch);
            TransactionResponse::Success(ExecutionData::EpochChange)
        } else {
            self.committee_info.set(current_epoch, current_committee);
            TransactionResponse::Success(ExecutionData::None)
        }
    }

    fn add_service(
        &self,
        sender: PublicKey,
        service: Service,
        service_id: ServiceId,
    ) -> TransactionResponse {
        // TODO: Verify that the sender is either Governance or Owner public key
        // TODO: Passing in service_id is bad and should be changed in future
        // This whole function is unsafe and going to change before this interface is finished
        self.services.set(service_id, service);
        TransactionResponse::Success(ExecutionData::None)
    }

    fn remove_service(&self, sender: PublicKey, service_id: ServiceId) -> TransactionResponse {
        todo!()
    }

    fn slash(
        &self,
        sender: PublicKey,
        proof: ProofOfMisbehavior,
        service_id: ServiceId,
        node: PublicKey,
    ) -> TransactionResponse {
        todo!()
    }

    /*******External View Functions*******/
    // The following functions should be called through execute_txn as the result of a txn
    // They will never change state even if called through update
    // Will usually only be called through query calls where msg.sender is not checked
    //      so if that is required for the function it should be made a parameter instead

    fn get_flk(&self, account: PublicKey) -> TransactionResponse {
        let balance = self
            .account_info
            .get(&account)
            .unwrap_or_default()
            .flk_balance;
        TransactionResponse::Success(ExecutionData::UInt(balance))
    }
    fn get_locked(&self, account: PublicKey) -> TransactionResponse {
        let balance = self
            .account_info
            .get(&account)
            .unwrap_or_default()
            .staking
            .locked;
        TransactionResponse::Success(ExecutionData::UInt(balance))
    }
    fn get_bandwidth(&self, account: PublicKey) -> TransactionResponse {
        let balance = self
            .account_info
            .get(&account)
            .unwrap_or_default()
            .bandwidth_balance;
        TransactionResponse::Success(ExecutionData::UInt(balance))
    }
    fn get_staked(&self, node: PublicKey) -> TransactionResponse {
        if let Some(info) = self.node_info.get(&node) {
            let staked = self
                .account_info
                .get(&info.owner)
                .unwrap_or_default()
                .staking
                .staked;
            TransactionResponse::Success(ExecutionData::UInt(staked))
        } else {
            TransactionResponse::Success(ExecutionData::UInt(0))
        }
    }
    fn get_reward_pool(&self, epoch: Epoch) -> TransactionResponse {
        let reward_pool = self
            .bandwidth_info
            .get(&epoch)
            .unwrap_or_default()
            .reward_pool;
        TransactionResponse::Success(ExecutionData::UInt(reward_pool))
    }
    fn get_total_served(&self, epoch: Epoch) -> TransactionResponse {
        let total_served = self
            .bandwidth_info
            .get(&epoch)
            .unwrap_or_default()
            .total_served;
        TransactionResponse::Success(ExecutionData::UInt(total_served))
    }
    fn get_node_bandwidth_served(&self, epoch: Epoch, node: PublicKey) -> TransactionResponse {
        let served = self
            .bandwidth_info
            .get(&epoch)
            .unwrap_or_default()
            .bandwidth_per_node
            .get(&node)
            .unwrap_or(&0)
            .to_owned();
        TransactionResponse::Success(ExecutionData::UInt(served))
    }
    fn get_current_epoch_info(&self) -> TransactionResponse {
        let epoch = self.metadata.get(&Metadata::Epoch).unwrap_or_default();
        let committee = self.committee_info.get(&epoch).unwrap_or_default();

        let vec = committee
            .members
            .iter()
            // Safe unwrap, a node should never be added to committee unless we have all data
            // These checks should be done when adding to the committee and to the whitelist.
            .map(|node| self.node_info.get(node).unwrap())
            .collect();

        TransactionResponse::Success(ExecutionData::EpochInfo(EpochInfo {
            committee: vec,
            epoch,
            epoch_end: committee.epoch_end_timestamp,
        }))
    }
    /********Internal Application Functions*********/
    // These functions should only ever be called in the context of an external transaction function
    // They should never panic and any check that could result in that should be done in the external function that calls it
    // The functions that should call this and the required checks should be documented for each function

    // This function should be called during signal_epoch_change.
    fn distribute_rewards(&self) {
        todo!()
    }
    fn choose_new_committee(&self) -> Vec<PublicKey> {
        // Todo
        // we need true randomness here, for now we will return the same committee.
        let epoch = self.metadata.get(&Metadata::Epoch).unwrap_or_default();
        self.committee_info.get(&epoch).unwrap_or_default().members
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    /// The sender of the transaction. In updates this is verified against the signature
    pub sender: PublicKey,
    /// The nonce of the account. It is incremented after every transaction to prevent replay attacks
    pub nonce: u128,
    /// The type of transaction that will be executed by this transaction
    pub transaction_type: TransactionType,
    /// The signature. Optional because querys do not need one.
    pub signature: Option<Signature>,
}

impl Transaction {
    /// Returns a transaction with default fields for everything besides transactionType
    pub fn get_query(transaction_type: TransactionType) -> Self {
        Self {
            sender: PublicKey::from_bytes(&[0; 32]).unwrap(),
            nonce: 0,
            transaction_type,
            signature: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionType {
    /// The main function of the application layer. After serving a client a node will submit this
    ///     transaction to get paid.
    ProofOfDelivery {
        /// The client that was served
        client: PublicKey,
        /// How much of the commodity was served
        commodity: u128,
        /// The service id of the service this was provided through(CDN, compute, ect.)
        service_id: u64,
        /// The PoD of delivery in bytes
        proof: (),
    },
    /// Withdraw tokens from the network back to the L2
    Withdraw {
        /// The amount to withdrawl
        amount: u128,
        /// Which token to withdrawl
        token: Tokens,
        /// The address to recieve these tokens on the L2
        receiving_address: PublicKey,
    },
    /// Submit of PoC from the bridge on the L2 to get the tokens in network
    Deposit {
        /// The proof of the bridge recieved from the L2,
        proof: ProofOfConsensus,
        /// Which token was bridged
        token: Tokens,
        /// Amount bridged
        amount: u128,
    },
    /// Stake FLK in network
    Stake {
        /// The proof of the bridge recieved from the L2,
        proof: ProofOfConsensus,
        /// Amount bridged
        amount: u128,
        /// Node Public Key
        node: PublicKey,
    },
    /// Unstake FLK, the tokens will be locked for a set amount of time(ProtocolParameter::LockTime) before they can be withdrawn
    Unstake { amount: u128 },
    /// Sent by committee member to signal he is ready to change epoch
    ChangeEpoch,
    /// Adding a new service to the protocol
    AddService {
        service: Service,
        service_id: ServiceId,
    },
    /// Removing a service from the protocol
    RemoveService {
        /// Service Id of the service to be removed
        service_id: ServiceId,
    },
    /// Provide proof of misbehavior to slash a node
    Slash {
        /// Service id of the service a node misbehaved in
        service_id: ServiceId,
        /// The public key of the node that misbehaved
        node: PublicKey,
        /// Zk proof to be provided to the slash circuit
        proof_of_misbehavior: ProofOfMisbehavior,
    },
    /// Querys do not change state
    Query(Query),
}

///Transactions that dont change state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Query {
    /// Get the balance of unlocked FLK a public key has
    FLK { public_key: PublicKey },
    /// Get the balance of locked FLK a public key has
    Locked { public_key: PublicKey },
    /// Get the amount of prepaid bandwidth a public key has
    Bandwidth { public_key: PublicKey },
    /// Get the amount of stake a node has
    Staked { node: PublicKey },
    /// Get the amound of bandwidth served in an epoch
    Served {
        /// the epoch
        epoch: Epoch,
        /// The node public Key
        node: PublicKey,
    },
    /// Get the total served for all nodes in an epoch
    TotalServed { epoch: Epoch },
    /// Get the amount in the reward pool for an epoch
    RewardPool { epoch: Epoch },
    /// Get the current epoch information
    CurrentEpochInfo,
}

#[derive(Clone, Debug)]
pub enum TransactionResponse {
    Success(ExecutionData),
    Revert(ExecutionError),
}

impl TransactionResponse {
    /// If response contains a Uint will return
    pub fn to_number(&self) -> Result<u128, TransactionResponse> {
        if let TransactionResponse::Success(ExecutionData::UInt(num)) = self {
            Ok(*num)
        } else {
            Err(self.clone())
        }
    }
}

#[derive(Clone, Debug)]
pub enum ExecutionData {
    None,
    String(String),
    UInt(u128),
    EpochInfo(EpochInfo),
    EpochChange,
}

#[derive(Clone, Debug)]
pub enum ExecutionError {
    InvalidSignature,
    InvalidNonce,
    InvalidProof,
    NotNodeOwner,
    NotCommitteeMember,
    NodeDoesNotExist,
    AlreadySignaled,
    NonExistingService,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Tokens {
    FLK,
    USDC,
}

pub trait Backend {
    type Ref<K: Eq + Hash + Send + Serialize + DeserializeOwned + 'static, V: Clone + Send + Serialize + DeserializeOwned + 'static>: TableRef<K, V>;

    fn get_table_reference<
        K: Eq + Hash + Send + Serialize + DeserializeOwned,
        V: Clone + Send + Serialize + DeserializeOwned,
    >(
        &self,
        id: &str,
    ) -> Self::Ref<K, V>;
    /// This function takes in the Transaction and verifies the Signature matches the Sender. It also checks the nonce
    /// of the sender and makes sure it is equal to the account nonce + 1, to prevent replay attacks and enforce ordering
    fn verify_transaction(&self, txn: &Transaction) -> Result<(), ExecutionError>;
    /// Takes in a zk Proof Of Delivery and returns true if valid
    fn verify_proof_of_delivery(
        &self,
        client: &PublicKey,
        provider: &PublicKey,
        commodity: &u128,
        service_id: &u64,
        proof: (),
    ) -> bool;
    /// Takes in a zk Proof Of Consensus and returns true if valid
    fn verify_proof_of_consensus(&self, proof: ProofOfConsensus) -> bool;
    /// Takes in a zk Proof Of Misbehavior and returns true if valid
    fn verify_proof_of_misbehavior(&self, proof: ProofOfMisbehavior) -> bool;
}

pub trait TableRef<K, V> {
    fn set(&self, key: K, value: V);
    fn get(&self, key: &K) -> Option<V>;
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AccountInfo {
    pub flk_balance: u128,
    pub bandwidth_balance: u128,
    pub nonce: u128,
    pub staking: Staking,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Staking {
    pub staked: u128,
    pub locked: u128,
    pub locked_until: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NodeInfo {
    pub owner: PublicKey,
    pub public_key: BLSPublicKey,
    pub network_key: NetworkKey,
    pub domain: multiaddr::Multiaddr,
    pub workers: Vec<Worker>,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct EpochInfo {
    pub committee: Vec<NodeInfo>,
    pub epoch: Epoch,
    pub epoch_end: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Worker {
    pub public_key: NetworkKey,
    pub address: multiaddr::Multiaddr,
    pub mempool: multiaddr::Multiaddr,
    //Do we need to store id here?
    // pub id: u64,
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ProtocolParams {
    /// The time in seconds that an epoch lasts for. Genesis 24 hours(86400)
    EpochTime = 0,
    /// The size of the committee
    CommitteeSize = 1,
    /// The min FLK a node has to stake to participate in the network
    MinimumNodeStake = 2,
    /// The time in epochs a node has to be staked to participate in the network
    EligibilityTime = 3,
    /// The time in epochs a node has to wait to withdraw after unstaking
    LockTime = 4,
    /// The percentage of the reward pool the protocol gets
    ProtocolPercentage = 5,
    /// The maximum targed inflation rate in a year
    MaxInflation = 6,
    /// The minimum targeted inflation rate in a year
    MinInflation = 7,
    /// The amount of FLK minted per GB they consume.
    ConsumerRebate = 8,
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Metadata {
    Epoch,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BandwidthInfo {
    pub total_served: u128,
    pub reward_pool: u128,
    pub bandwidth_per_node: HashMap<PublicKey, u128>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Committee {
    pub members: Vec<PublicKey>,
    pub ready_to_change: Vec<PublicKey>,
    pub epoch_end_timestamp: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Service {
    pub commodity_price: u128,
    /// TODO: List of circuits to prove a node should be slashed
    pub slashing: (),
}

///Placeholder
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ProofOfConsensus {}

///Placeholder
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ProofOfMisbehavior {
    /// Service Id of the service that a node misbehaved in.
    pub service_id: ServiceId,
    /// Todo: The circuit that proves a node should be slashed
    pub proof: (),
}

/// Placeholder
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Signature();
