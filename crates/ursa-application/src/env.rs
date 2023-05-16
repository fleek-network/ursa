use crate::backend::AtomoBackend;
use crate::interface::application::{
    AccountInfo, ApplicationState, BandwidthInfo, Committee, Env, Epoch, Metadata, NodeInfo,
    ProtocolParams, PublicKey, QueryEnv, Service, ServiceId, Transaction, TransactionResponse,
};
use atomo::mt::{MtAtomo, MtAtomoBuilder, QueryPerm, UpdatePerm};
use atomo::DefaultSerdeBackend;

pub struct AtomoEnv<P> {
    atomo: MtAtomo<P>,
}

impl AtomoEnv<UpdatePerm> {
    pub fn new() -> Self {
        let atomo = MtAtomoBuilder::<DefaultSerdeBackend>::new()
            .with_table::<Metadata, u64>("metadata")
            .with_table::<PublicKey, AccountInfo>("account")
            .with_table::<PublicKey, NodeInfo>("node")
            .with_table::<Epoch, Committee>("committee")
            .with_table::<Epoch, BandwidthInfo>("bandwidth")
            .with_table::<ServiceId, Service>("service")
            .with_table::<ProtocolParams, u128>("parameter")
            .build();

        Self { atomo }
    }
}

impl Env for AtomoEnv<UpdatePerm> {
    type Query = AtomoEnv<QueryPerm>;

    fn run(&mut self, transaction: Transaction) -> TransactionResponse {
        self.atomo.run(move |ctx| {
            let backend = AtomoBackend {
                table_selector: ctx,
            };

            let app = ApplicationState::new(backend);

            app.execute_txn(transaction.clone())
        })
    }

    fn query(&self) -> Self::Query {
        AtomoEnv {
            atomo: self.atomo.query(),
        }
    }
}

impl QueryEnv for AtomoEnv<QueryPerm> {
    fn run(&self, transaction: Transaction) -> TransactionResponse {
        self.atomo.run(|ctx| {
            let backend = AtomoBackend {
                table_selector: ctx,
            };
            let app = ApplicationState::new(backend);
            app.execute_txn(transaction.clone())
        })
    }
}
