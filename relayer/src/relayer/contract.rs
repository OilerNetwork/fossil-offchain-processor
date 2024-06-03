// use std::str::FromStr;
//
// use primitive_types::U256;
// use web3::{
//     contract::{tokens::Detokenize, Contract, Options},
//     ethabi::{Address, Token},
//     transports::Http,
// };
//
// pub struct TestContract(Contract<Http>);
//
// #[derive(Debug)]
// struct EmptyBody {}
//
// impl Detokenize for EmptyBody {
//     fn from_tokens(tokens: Vec<Token>) -> Result<Self, web3::contract::Error>
//     where
//         Self: Sized,
//     {
//         dbg!(tokens);
//         Ok(EmptyBody {})
//     }
// }
//
// impl TestContract {
//     pub async fn new(web3: &web3::Web3<web3::transports::Http>, address: String) -> Self {
//         let address = Address::from_str(&address).unwrap();
//         let contract =
//             Contract::from_json(web3.eth(), address, include_bytes!("L1_MessageSender.json"))
//                 .unwrap();
//         TestContract(contract)
//     }
//
//     pub async fn send_latest_parent_has_to_l2(&self) {
//         let result: Result<EmptyBody, web3::contract::Error> = self
//             .0
//             .query(
//                 "sendExactParentHashToL2",
//                 (1),
//                 None,
//                 Options::default(),
//                 None,
//             )
//             .await;
//         dbg!(result);
//     }
// }
