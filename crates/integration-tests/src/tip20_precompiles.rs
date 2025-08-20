#[cfg(test)]
mod tests {
    use tempo_precompiles::contracts::ITIP20;
    use crate::{get_local_provider, get_prefunded_addresses};

    #[tokio::test]
    pub async fn prefunded_address_has_tip20_tokens() {
        let provider = get_local_provider().unwrap();
        let tip20_address = crate::PREDEPLOYED_TIP20_ADDRESS.parse().unwrap();
        let prefunded_addresses = get_prefunded_addresses(50).unwrap();

        let token_precompile = ITIP20::new(tip20_address, &provider);

        for address in prefunded_addresses {
            assert_ne!(token_precompile.balanceOf(address).call().await.unwrap(), 0);
        }
    }
}