# Narwhal and Bullshark - Mempool and Consensus

> Narwhal and Bullshark (N/B) - Fleek Network's consensus and ordering algorithms. A DAG based consensus with total ordering and reliable broadcast.

This code has been adapted from the [MystenLabs Sui](https://github.com/MystenLabs/sui)

important in this crate: 

abci_engine.rs - this file is the engine that drives the application layer from consensus. It listens to 2 channels:
- rx_certificates: Narwhal execution engine has the other end of this channel. It sends all Certficates from narwhal through this channel. When the abci_engine gets a certificate from this channel, it calls the abci lifecycle methods through the abci client. begin_block, deliver_tx(for each tx in the certificate), end_block, and then commit. The engine does not care about the results of any transaction it executes besides one scenario; when the application layer says the results of the txn says to change epoch. When this happens it uses a channel to communicate this to the Consensus struct.
- rx_abci_queries: Queries to the application layer are sent through this channel. They cannot change state. Queries go directly to the application layer and not through consensus. Senders to this channel can be passed around ursa to whatever needs to query information from the application state.

consensus.rs - This is the entry point of the consensus. The consensus stuct in this file wraps everything to do with narwhal besides the ABCI Engine. It listens for the signal for epoch change and when it gets it, it shutdowns narwhal, pulls new epoch info, and starts of a new epoch specific instance of narwhal