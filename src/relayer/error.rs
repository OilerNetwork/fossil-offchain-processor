use ethers::providers::Middleware;

#[derive(Debug)]
pub enum RelayerError<M: Middleware> {
    Abi(ethers::core::abi::Error),
    Provider(ethers::providers::ProviderError),
    Middleware(<M as Middleware>::Error),
}

impl<M: Middleware + 'static> From<ethers::core::abi::Error> for RelayerError<M> {
    fn from(e: ethers::core::abi::Error) -> Self {
        Self::Abi(e)
    }
}

impl<M: Middleware + 'static> From<ethers::providers::ProviderError> for RelayerError<M> {
    fn from(e: ethers::providers::ProviderError) -> Self {
        Self::Provider(e)
    }
}
