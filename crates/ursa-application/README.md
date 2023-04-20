# Ursa application layer

Evm based application layer for ursa. The abci server is how this application is interacted with. There are a few different connections the abci server offers to our application.

Consensus - This connection contains write access to the state. All state changing transactions go through this connection. We only forward transactions that are included on Narwhal Certificates through this connection. This connection has the following methods:

- init_chain - we call this method to initialize the genesis state on the chain, we load in our genesis contracts here. Is only called once and abci will return an err(which we ignore in consensus) if it is called after already init
- begin_block - Signals the beginning of a new block called before any deliver_tx. We currently dont do anything here, and only use it to conform to abci spec. But if we add any logic that needs to happen before executions of transactions in a certificate it would go here.
- deliver_tx - this executes a transaction against current_state, does not commit the changes but holds them in memory, which any following transactions in this block will execute against.
- end_block - Ends the block. Called after all deliver_txs and before commit. Currently we only increment block height here.
- commit- we call this after end_block to persist the changes in the state. This is the final lifecycle method we call. Before this is called any queries going to the application layer read against the state from the last commit. This is so no queries read state while its mid transition.

Info - This connection contains read only access to the state and it only reads the last committed state. One function, query, we use that executes a transaction and returns the result and does not commit state. We do not need to go through consensus to use this connection and nodes use this connection to query their application state directly

For more info on the ABCI spec we are conforming too check out -[https://github.com/tendermint/tendermint/blob/v0.34.x/spec/abci/README.md](https://github.com/tendermint/tendermint/blob/v0.34.x/spec/abci/README.md)

Epoch changes on the consensus layer are based on the state of the Epoch contract being loaded in at genesis. This contract auto updates the next epoch end time based on its parameters. This makes devolopment hard so ive included a bin in this crate that sets the first epoch start time to the current time. It also takes an optional parameter to set length of epoch(in Ms) if you dont specify it defaults to 5 minutes. example of running the bin and setting epoch to 10 minutes looks like this
`cargo run --bin genesis 600000`

This bin also sets the default committee to the nodes in this repo- https://github.com/qti3e/ursa-nodes. To test application/consensus i recommend using that repo and running the bin right before you start the nodes.

files in this crate:

- app.rs - Builds the application struct that the server hosts
- config.rs - loads the application specific config. Currently only is domain the server can be reached at
- genesis.rs - loads in the genesis config for the init_chain function
- server.rs - starting the abci server the application is interacted with through
- types.rs - Currently most of the logic for application lives here. We implement the traits each abci connection needs here. All of these traits are added onto the App struct, in app.rs.

### Precompiles

This crate loads in precompile contracts at the following addresses

ProtocolParameters - 0x0000000000000000000000000000000000000099
FLK Token - 0x0000000000000000000000000000000000000098
Staking - 0x0000000000000000000000000000000000000097
NodeRegistry - 0x0000000000000000000000000000000000000096
EpochManager - 0x0000000000000000000000000000000000000095
RewardsManager - 0x0000000000000000000000000000000000000094
RewardsAggregator - 0x0000000000000000000000000000000000000093
ReputationScores - 0x0000000000000000000000000000000000000092
