pub use evm::backend::{ApplyBackend, Backend};

use crate::types::{Account, Bytes, ExecResp, Log, MerkleRoot, SignedTransaction, TxResp, H160};

pub trait ExecutorAdapter {
    fn get_logs(&self) -> Vec<Log>;

    fn state_root(&self) -> MerkleRoot;

    fn get(&self, key: &[u8]) -> Option<Bytes>;
}

pub trait Executor: Send + Sync {
    fn call<B: Backend>(&self, backend: &mut B, addr: H160, data: Vec<u8>) -> TxResp;

    fn exec<B: Backend + ApplyBackend + ExecutorAdapter>(
        &self,
        backend: &mut B,
        txs: Vec<SignedTransaction>,
    ) -> ExecResp;

    fn get_account<B: Backend + ExecutorAdapter>(&self, backend: &B, address: &H160) -> Account;
}