use txline::config::{
    DEVNET_API_BASE, DEVNET_API_HOST, DEVNET_GUEST_AUTH_URL, DEVNET_PROGRAM_ID, DEVNET_RPC_URL,
    DEVNET_TXL_MINT, DEVNET_USDT_MINT,
};
use txline::solana::pda::DevnetPdas;
use txline::solana::subscription::validate_subscription_weeks;
use txline::{Network, TxlineClient, TxlineConfig};

#[test]
fn devnet_config_constants_are_canonical() {
    let cfg = TxlineConfig::devnet();
    assert_eq!(cfg.network, Network::Devnet);
    assert_eq!(cfg.api_host, DEVNET_API_HOST);
    assert_eq!(cfg.api_base, DEVNET_API_BASE);
    assert_eq!(cfg.guest_auth_url, DEVNET_GUEST_AUTH_URL);
    assert_eq!(cfg.program_id, DEVNET_PROGRAM_ID);
    assert_eq!(cfg.txl_mint, DEVNET_TXL_MINT);
    assert_eq!(cfg.usdt_mint, DEVNET_USDT_MINT);
    assert_eq!(cfg.rpc_url, DEVNET_RPC_URL);
}

#[test]
fn custom_devnet_rpc_urls_are_allowed() {
    for rpc_url in [
        "https://api.devnet.solana.com",
        "https://devnet.helius-rpc.com/?api-key=test",
        "https://custom-rpc.example.com/solana/devnet",
    ] {
        let client = TxlineClient::new(TxlineConfig::devnet().with_rpc_url(rpc_url)).unwrap();
        assert_eq!(client.config().rpc_url, rpc_url);
    }
}

#[test]
fn obvious_mainnet_rpc_urls_are_rejected() {
    for rpc_url in [
        "https://api.mainnet-beta.solana.com",
        "https://mainnet.helius-rpc.com/?api-key=test",
        "https://rpc.example.com/solana/mainnet",
    ] {
        let err = TxlineClient::new(TxlineConfig::devnet().with_rpc_url(rpc_url)).unwrap_err();
        assert!(err.to_string().contains("Devnet RPC endpoint"));
    }
}

#[test]
fn derives_known_devnet_pdas() {
    let pdas = DevnetPdas::new().unwrap();
    assert_eq!(
        pdas.pricing_matrix().address.to_string(),
        "B4hHn1FpD1YPPrcM4yUrQhBPF18zFWgijHLTsumGzeKi"
    );
    assert_eq!(
        pdas.token_treasury_v2().address.to_string(),
        "Eqqd7rZQGzn2HA9L11NwBMhknxArM3L4KETyUuujK3LB"
    );
    assert_eq!(
        pdas.token_treasury_vault_ata().unwrap().address.to_string(),
        "dc6rQSPk8GJAeyyAtC1F62JoigmgEuLnW4k9zmgAeuM"
    );
    assert_eq!(
        pdas.daily_scores_roots(20_624).address.to_string(),
        "BM2n3RE2ADwZDaGehqd4mtkCQsFgW4aQrtdnDQ3VR4Kn"
    );
    assert_eq!(
        pdas.daily_batch_roots(20_624).address.to_string(),
        "2Y3dpLFRjA9M6J4MiAgrZ7AkPfzfke8Fzm2GLU92vTja"
    );
    assert_eq!(
        pdas.ten_daily_fixtures_roots(20_629).address.to_string(),
        "2ATJ2TkoB1c2PfTUqfDun1SwvnRNTTyGBt7D9h9Vkccd"
    );
}

#[test]
fn validates_subscription_duration_rules() {
    assert!(validate_subscription_weeks(4).is_ok());
    assert!(validate_subscription_weeks(8).is_ok());
    assert!(validate_subscription_weeks(0).is_err());
    assert!(validate_subscription_weeks(3).is_err());
    assert!(validate_subscription_weeks(5).is_err());
}
