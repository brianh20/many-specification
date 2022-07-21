use super::LedgerConfig;
use crate::helpers::{generate_key, has_attribute, message, send};
use crate::tests::{ReadConfig, TestCaseResult, TestConfig};
use ciborium::value::Value;
use many_identity::Address;
use many_types::ledger::{Symbol, TokenAmount};
use minicbor::{Decode, Encode};
use reftests_macros::test_case;
use std::collections::BTreeMap;

pub struct LedgerClient {
    pub test_config: TestConfig,
    pub ledger_config: LedgerConfig,
}

#[derive(Clone, Encode, Decode)]
#[cbor(map)]
pub struct BalanceReturns {
    #[n(0)]
    pub balances: BTreeMap<Symbol, TokenAmount>,
}

impl LedgerClient {
    pub async fn new(
        test_config: TestConfig,
        attributes: Option<Vec<u32>>,
    ) -> Result<Self, String> {
        let envelope = message("status", "null", None, None);
        let _response = send(&test_config, envelope).await;

        if attributes.is_some() {
            for a in attributes.unwrap().into_iter() {
                if !has_attribute(a, &test_config).await {
                    return Err("Server does not support ledger attribute.".to_string());
                }
            }
        }

        let ledger_config = test_config.read_config("ledger".to_string());

        Ok(Self {
            test_config,
            ledger_config,
        })
    }

    pub fn get_identity(&self, key_seed: u8) -> Address {
        let (_, kid, _) = generate_key(Some(key_seed), None);
        Address::from_bytes(&kid).unwrap()
    }

    pub async fn balance(&self, account: String, symbol: Address) -> Result<u128, String> {
        let payload = format!("{{0:\"{}\", 1:\"{}\"}}", account, symbol);
        let envelope = message("ledger.balance", payload, None, None);
        let response = send(&self.test_config, envelope).await;

        let payload = response.payload.expect("No payload from status");
        let response: BTreeMap<u8, Value> =
            ciborium::de::from_reader(payload.as_slice()).expect("Invalid payload.");

        let response_payload = response
            .get(&4)
            .expect("Response return value was not found")
            .as_bytes()
            .ok_or_else(|| {
                format!(
                    "[Response: {:?} -> was expected to be Bytes]",
                    response.get(&4).unwrap()
                )
            })?;

        let balance_returns: BalanceReturns =
            minicbor::decode(response_payload.as_slice()).unwrap();

        let balance = balance_returns.balances.get(&symbol);

        if balance == None {
            return Ok(0);
        }

        Ok(balance.unwrap().to_string().parse::<u128>().unwrap())
    }

    pub async fn send(
        &self,
        key_seed: Option<u8>,
        from: String,
        to: String,
        amount: u64,
        symbol: Symbol,
        pem: Option<String>,
    ) -> Result<(), String> {
        let payload = format!(
            "{{0:\"{}\", 1:\"{}\", 2:{}, 3:\"{}\"}}",
            from,
            to,
            TokenAmount::from(amount),
            symbol
        );

        let envelope = if let Some(pem) = pem {
            message("ledger.send", payload, None, Some(pem))
        } else {
            message("ledger.send", payload, key_seed, None)
        };

        let response = send(&self.test_config, envelope).await;

        let payload = response.payload.expect("No payload from status");
        let response: BTreeMap<u8, Value> =
            ciborium::de::from_reader(payload.as_slice()).expect("Invalid payload.");

        let _ = response
            .get(&4)
            .expect("Response return value was not found")
            .as_bytes()
            .ok_or_else(|| {
                format!(
                    "[Response: {:?} -> was expected to be Bytes]",
                    response.get(&4).unwrap()
                )
            })?;

        Ok(())
    }
}

// #[test_case]
// async fn send_to_bad_identity(test_config: TestConfig) -> TestCaseResult {
//     let client = LedgerClient::new(test_config, Some(vec![2, 6]))
//         .await
//         .unwrap();

//     async fn send_test(client: LedgerClient) -> Result<(), String> {
//         let akward_identity = "some_akward_impossible_identity".to_string();

//         // Checking faucet has enough funds for following tests.
//         assert!(
//             client
//                 .balance(
//                     client.ledger_config.faucet.to_string(),
//                     client.ledger_config.symbol
//                 )
//                 .await
//                 .unwrap()
//                 > 10000
//         );

//         client
//             .send(
//                 None,
//                 client.ledger_config.faucet.to_string(),
//                 akward_identity.to_owned(),
//                 10000,
//                 client.ledger_config.symbol,
//                 Some(client.ledger_config.faucet_pk.to_owned()),
//             )
//             .await
//             .map_or(Ok(()), |_| {
//                 Err("Was able to send from faucet to odd identity".to_string())
//             })
//     }

//     match send_test(client).await {
//         Ok(_) => TestCaseResult::Success(),
//         Err(s) => TestCaseResult::Skip(s),
//     }
// }

// // #[test_case]
// // async fn cant_send_with_bad_private_key(test_config: TestConfig) -> TestCaseResult {}
// // Skipped because the helper will panic if it can decode the PEM string

// #[test_case]
// async fn cant_send_without_funds(test_config: TestConfig) -> TestCaseResult {
//     let client = LedgerClient::new(test_config, Some(vec![2, 6]))
//         .await
//         .unwrap();

//     async fn send_test(client: LedgerClient) -> Result<(), String> {
//         // Checking faucet has enough funds for following tests.
//         assert!(
//             client
//                 .balance(
//                     client.ledger_config.faucet.to_string(),
//                     client.ledger_config.symbol
//                 )
//                 .await
//                 .unwrap()
//                 > 10000
//         );

//         // Getting Identity for seed: 1
//         let id1 = client.get_identity(1);

//         // Checking balance is 0
//         assert_eq!(
//             client
//                 .balance(id1.to_string(), client.ledger_config.symbol)
//                 .await
//                 .unwrap(),
//             0
//         );

//         // Getting Identity for seed: 2
//         let id2 = client.get_identity(2);

//         // Checking balance on Identity:2
//         assert!(
//             client
//                 .balance(id2.to_string(), client.ledger_config.symbol)
//                 .await
//                 .unwrap()
//                 == 0
//         );

//         // Sending 10000 from Identity:1 to Identity:2
//         client
//             .send(
//                 Some(1),
//                 id1.to_string(),
//                 id2.to_string(),
//                 10000,
//                 client.ledger_config.symbol,
//                 None,
//             )
//             .await
//             .map_or(
//                 Ok(()),
//                 |_| Err("Was able to send without funds".to_string()),
//             )
//     }

//     match send_test(client).await {
//         Ok(_) => TestCaseResult::Success(),
//         Err(s) => TestCaseResult::Skip(s),
//     }
// }

#[test_case]
async fn can_send_with_funds(test_config: TestConfig) -> TestCaseResult {
    let client = LedgerClient::new(test_config, Some(vec![2, 6]))
        .await
        .unwrap();

    async fn send_test(client: LedgerClient) -> Result<(), String> {
        // Checking faucet has enough funds for following tests.
        assert!(
            client
                .balance(
                    client.ledger_config.faucet.to_string(),
                    client.ledger_config.symbol
                )
                .await
                .unwrap()
                > 10000
        );

        // Getting Identity for seed: 1
        let id1 = client.get_identity(1);

        // Checking balance is 0
        assert_eq!(
            client
                .balance(id1.to_string(), client.ledger_config.symbol)
                .await
                .unwrap(),
            0
        );

        // Sending 10000 from faucet to Identity:1
        client
            .send(
                None,
                client.ledger_config.faucet.to_string(),
                id1.to_string(),
                10000,
                client.ledger_config.symbol,
                Some(client.ledger_config.faucet_pk.to_owned()),
            )
            .await
            .map_err(|e| format!("Was not able to send from faucet to id1: {}", e))?;

        // Checking balance on Identity:1
        assert!(
            client
                .balance(id1.to_string(), client.ledger_config.symbol)
                .await
                .unwrap()
                == 10000
        );

        // Getting Identity for seed: 2
        let id2 = client.get_identity(2);

        // Checking balance on Identity:2
        assert_eq!(
            client
                .balance(id2.to_string(), client.ledger_config.symbol)
                .await
                .unwrap(),
            0
        );

        // Sending 10000 from Identity:1 to Identity:2
        client
            .send(
                Some(1),
                id1.to_string(),
                id2.to_string(),
                10000,
                client.ledger_config.symbol,
                None,
            )
            .await
            .map_err(|e| format!("Was not able to send from id1 to id2: {}", e))?;

        // Checking Identity:1 balance is now 0
        assert!(
            client
                .balance(id1.to_string(), client.ledger_config.symbol)
                .await
                .unwrap()
                == 0
        );

        // Checking Identity:2 balance is now 10000
        assert_eq!(
            client
                .balance(id2.to_string(), client.ledger_config.symbol)
                .await
                .unwrap(),
            10000
        );

        // Sending funds from Identity:2 back to faucet
        client
            .send(
                Some(2),
                id2.to_string(),
                client.ledger_config.faucet.to_string(),
                10000,
                client.ledger_config.symbol,
                None,
            )
            .await
            .map_err(|e| format!("Was not able to send from identity2 to faucet: {}", e))?;

        // Checking Identity:2 balance is now 0
        assert!(
            client
                .balance(id2.to_string(), client.ledger_config.symbol)
                .await
                .unwrap()
                == 0
        );
        Ok(())
    }

    match send_test(client).await {
        Ok(_) => TestCaseResult::Success(),
        Err(s) => TestCaseResult::Skip(s),
    }
}

#[test_case]
async fn cant_get_balance_from_bad_identity(test_config: TestConfig) -> TestCaseResult {
    let client = LedgerClient::new(test_config, Some(vec![2])).await.unwrap();

    async fn balance_test(client: LedgerClient) -> Result<(), String> {
        let akward_identity = "some_akward_impossible_identity".to_string();

        // Checking balance for odd identity
        client
            .balance(akward_identity, client.ledger_config.symbol)
            .await
            .map_or(Ok(()), |_| {
                Err("Was able to get balance from bad identity".to_string())
            })
    }

    match balance_test(client).await {
        Ok(_) => TestCaseResult::Success(),
        Err(s) => TestCaseResult::Skip(s),
    }
}

#[test_case]
async fn can_get_balance_for_anonymous(test_config: TestConfig) -> TestCaseResult {
    let client = LedgerClient::new(test_config, Some(vec![2])).await.unwrap();

    async fn balance_test(client: LedgerClient) -> Result<(), String> {
        // Checking can get balance for anonymous.
        assert_eq!(
            client
                .balance("maaaa".to_string(), client.ledger_config.symbol)
                .await
                .unwrap(),
            0
        );
        Ok(())
    }

    match balance_test(client).await {
        Ok(_) => TestCaseResult::Success(),
        Err(s) => TestCaseResult::Skip(s),
    }
}

#[test_case]
async fn can_get_balance(test_config: TestConfig) -> TestCaseResult {
    let client = LedgerClient::new(test_config, Some(vec![2])).await.unwrap();

    async fn balance_test(client: LedgerClient) -> Result<(), String> {
        // Checking faucet has enough funds for following tests.
        assert!(
            client
                .balance(
                    client.ledger_config.faucet.to_string(),
                    client.ledger_config.symbol
                )
                .await
                .unwrap()
                > 10000
        );
        Ok(())
    }

    match balance_test(client).await {
        Ok(_) => TestCaseResult::Success(),
        Err(s) => TestCaseResult::Skip(s),
    }
}
