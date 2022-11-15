#![allow(missing_docs)]

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use async_trait::async_trait;
use ethers::prelude::*;
use eyre::Result;
use tracing::instrument;

use hyperlane_core::{
    HyperlaneAbi, HyperlaneChain, HyperlaneContract, ContractLocator, Indexer, InterchainGasPaymaster,
    InterchainGasPaymasterIndexer, InterchainGasPayment, InterchainGasPaymentMeta,
    InterchainGasPaymentWithMeta,
};

use crate::contracts::interchain_gas_paymaster::{
    InterchainGasPaymaster as EthereumInterchainGasPaymasterInternal, INTERCHAINGASPAYMASTER_ABI,
};
use crate::trait_builder::MakeableWithProvider;

impl<M> Display for EthereumInterchainGasPaymasterInternal<M>
where
    M: Middleware,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct InterchainGasPaymasterIndexerBuilder {
    pub mailbox_address: H160,
    pub finality_blocks: u32,
}

#[async_trait]
impl MakeableWithProvider for InterchainGasPaymasterIndexerBuilder {
    type Output = Box<dyn InterchainGasPaymasterIndexer>;

    async fn make_with_provider<M: Middleware + 'static>(
        &self,
        provider: M,
        locator: &ContractLocator,
    ) -> Self::Output {
        Box::new(EthereumInterchainGasPaymasterIndexer::new(
            Arc::new(provider),
            locator,
            self.mailbox_address,
            self.finality_blocks,
        ))
    }
}

#[derive(Debug)]
/// Struct that retrieves event data for an Ethereum InterchainGasPaymaster
pub struct EthereumInterchainGasPaymasterIndexer<M>
where
    M: Middleware,
{
    contract: Arc<EthereumInterchainGasPaymasterInternal<M>>,
    provider: Arc<M>,
    mailbox_address: H160,
    finality_blocks: u32,
}

impl<M> EthereumInterchainGasPaymasterIndexer<M>
where
    M: Middleware + 'static,
{
    /// Create new EthereumInterchainGasPaymasterIndexer
    pub fn new(
        provider: Arc<M>,
        locator: &ContractLocator,
        mailbox_address: H160,
        finality_blocks: u32,
    ) -> Self {
        Self {
            contract: Arc::new(EthereumInterchainGasPaymasterInternal::new(
                &locator.address,
                provider.clone(),
            )),
            provider,
            mailbox_address,
            finality_blocks,
        }
    }
}

#[async_trait]
impl<M> Indexer for EthereumInterchainGasPaymasterIndexer<M>
where
    M: Middleware + 'static,
{
    #[instrument(err, skip(self))]
    async fn get_finalized_block_number(&self) -> Result<u32> {
        Ok(self
            .provider
            .get_block_number()
            .await?
            .as_u32()
            .saturating_sub(self.finality_blocks))
    }
}

#[async_trait]
impl<M> InterchainGasPaymasterIndexer for EthereumInterchainGasPaymasterIndexer<M>
where
    M: Middleware + 'static,
{
    #[instrument(err, skip(self))]
    async fn fetch_gas_payments(
        &self,
        from_block: u32,
        to_block: u32,
    ) -> Result<Vec<InterchainGasPaymentWithMeta>> {
        let events = self
            .contract
            .gas_payment_filter()
            .topic1(self.mailbox_address)
            .from_block(from_block)
            .to_block(to_block)
            .query_with_meta()
            .await?;

        Ok(events
            .into_iter()
            .map(|(log, log_meta)| InterchainGasPaymentWithMeta {
                payment: InterchainGasPayment {
                    message_id: H256::from(log.message_id),
                    amount: log.amount,
                },
                meta: InterchainGasPaymentMeta {
                    transaction_hash: log_meta.transaction_hash,
                    log_index: log_meta.log_index,
                },
            })
            .collect())
    }
}

pub struct InterchainGasPaymasterBuilder {}

#[async_trait]
impl MakeableWithProvider for InterchainGasPaymasterBuilder {
    type Output = Box<dyn InterchainGasPaymaster>;

    async fn make_with_provider<M: Middleware + 'static>(
        &self,
        provider: M,
        locator: &ContractLocator,
    ) -> Self::Output {
        Box::new(EthereumInterchainGasPaymaster::new(
            Arc::new(provider),
            locator,
        ))
    }
}

/// A reference to an InterchainGasPaymaster contract on some Ethereum chain
#[derive(Debug)]
pub struct EthereumInterchainGasPaymaster<M>
where
    M: Middleware,
{
    #[allow(dead_code)]
    contract: Arc<EthereumInterchainGasPaymasterInternal<M>>,
    chain_name: String,
    #[allow(dead_code)]
    domain: u32,
    #[allow(dead_code)]
    provider: Arc<M>,
}

impl<M> EthereumInterchainGasPaymaster<M>
where
    M: Middleware + 'static,
{
    /// Create a reference to a outbox at a specific Ethereum address on some
    /// chain
    pub fn new(provider: Arc<M>, locator: &ContractLocator) -> Self {
        Self {
            contract: Arc::new(EthereumInterchainGasPaymasterInternal::new(
                &locator.address,
                provider.clone(),
            )),
            domain: locator.domain,
            chain_name: locator.chain_name.to_owned(),
            provider,
        }
    }
}

impl<M> HyperlaneChain for EthereumInterchainGasPaymaster<M>
where
    M: Middleware + 'static,
{
    fn chain_name(&self) -> &str {
        &self.chain_name
    }

    fn local_domain(&self) -> u32 {
        self.domain
    }
}

impl<M> HyperlaneContract for EthereumInterchainGasPaymaster<M>
where
    M: Middleware + 'static,
{
    fn address(&self) -> H256 {
        self.contract.address().into()
    }
}

#[async_trait]
impl<M> InterchainGasPaymaster for EthereumInterchainGasPaymaster<M> where M: Middleware + 'static {}

pub struct EthereumInterchainGasPaymasterAbi;

impl HyperlaneAbi for EthereumInterchainGasPaymasterAbi {
    fn fn_map() -> HashMap<Selector, &'static str> {
        super::extract_fn_map(&INTERCHAINGASPAYMASTER_ABI)
    }
}