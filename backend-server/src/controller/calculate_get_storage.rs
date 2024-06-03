use std::str::FromStr;

use axum::{
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue},
    response::IntoResponse,
    Json,
};
use primitive_types::U256;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::model::{
    account_proof::AccountProof,
    data::{Data, IntsSequence},
    hex::{u64_or_hex, HexString},
    storage_proof::StorageProof,
};

#[derive(Deserialize)]
pub struct Input {
    account_address: String,
    storage_keys: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct EthRpcBody {
    jsonrpc: String,
    method: String,
    params: Vec<EthRpcBodyParams>,
    id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum EthRpcBodyParams {
    AccountAddress(String),
    StorageKeys(Vec<String>),
    BlockIdentifier(String),
}

pub async fn call_mev_blocker_api(
    State(client): State<Client>,
    Json(input): Json<Input>,
) -> impl IntoResponse {
    let account_address = input.account_address;
    let storage_keys = input.storage_keys;

    let body = EthRpcBody {
        jsonrpc: "2.0".to_string(),
        method: "eth_getProof".to_string(),
        params: vec![
            EthRpcBodyParams::AccountAddress(account_address),
            EthRpcBodyParams::StorageKeys(storage_keys.clone()),
            EthRpcBodyParams::BlockIdentifier("latest".to_string()),
        ],
        id: "1".to_string(),
    };

    let url = dotenv::var("ETH_RPC").expect("ETH_RPC must be set");

    let reqwest_response = client
        .post(url)
        .json(&body)
        .headers(HeaderMap::from_iter(vec![
            (
                HeaderName::from_static("content-type"),
                HeaderValue::from_static("application/json"),
            ),
            (
                HeaderName::from_static("accept"),
                HeaderValue::from_static("application/json"),
            ),
        ]))
        .send()
        .await;

    match reqwest_response {
        Ok(res) => {
            let json = res
                .json::<serde_json::Value>()
                .await
                .expect("JSON was not well-formatted");

            match json {
                serde_json::Value::Object(map) => {
                    if let Some(trie_proofs) = map.get("result") {
                        // Check if storageProof is empty
                        if trie_proofs.get("storageProof").is_none()
                            || trie_proofs["storageProof"].as_array().unwrap().is_empty()
                        {
                            return (StatusCode::BAD_REQUEST, Json("storageProof is empty"))
                                .into_response();
                        }

                        match calculate_proof(trie_proofs, storage_keys) {
                            Ok(response_proof) => {
                                (StatusCode::OK, Json(response_proof)).into_response()
                            }
                            Err(err) => {
                                (StatusCode::BAD_REQUEST, Json(err.to_string())).into_response()
                            }
                        }
                    } else if let Some(error) = map.get("error") {
                        (
                            StatusCode::BAD_REQUEST,
                            Json(error.get("message").unwrap().to_string()),
                        )
                            .into_response()
                    } else {
                        (
                            StatusCode::BAD_REQUEST,
                            Json("Invalid response from eth_getProof API"),
                        )
                            .into_response()
                    }
                }
                _ => (StatusCode::BAD_REQUEST, Json("JSON was not well-formatted")).into_response(),
            }
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json("Something went wrong with the request"),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EthTrieProofs {
    address: U256,
    balance: U256,
    code_hash: U256,
    #[serde(deserialize_with = "u64_or_hex")]
    nonce: u64,
    storage_hash: U256,
    account_proof: Vec<String>,
    storage_proof: Vec<EthStorageProof>,
}

#[derive(Deserialize)]
struct EthStorageProof {
    key: U256,
    value: U256,
    proof: Vec<String>,
}

fn calculate_proof(
    trie_proof: &serde_json::Value,
    storage_keys: Vec<String>,
) -> Result<Proof, Box<dyn std::error::Error>> {
    let eth_trie_proofs = serde_json::from_value::<EthTrieProofs>(trie_proof.to_owned())?;

    if eth_trie_proofs.account_proof.is_empty() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "accountProof is empty",
        )));
    }

    let account_proof_ints_sequence: Vec<IntsSequence> = eth_trie_proofs
        .account_proof
        .iter()
        .map(|element| -> IntsSequence { Data::from(HexString::new(element)).into() })
        .collect();

    let mut flat_account_proof: Vec<u64> = Vec::new();
    let mut flat_account_proof_sized_bytes = Vec::new();
    let mut flat_account_proof_sized_words = Vec::new();

    for proof_element in account_proof_ints_sequence.into_iter() {
        let words_length = proof_element.values.len();
        flat_account_proof.extend(proof_element.values);
        flat_account_proof_sized_bytes.push(proof_element.length);
        flat_account_proof_sized_words.push(words_length);
    }

    let state_root: U256 = U256::from_str(&storage_keys[0])?;

    let len_proof = flat_account_proof.len();

    let account_proof = AccountProof {
        address: eth_trie_proofs.address,
        balance: eth_trie_proofs.balance,
        code_hash: eth_trie_proofs.code_hash,
        storage_hash: eth_trie_proofs.storage_hash,
        nonce: eth_trie_proofs.nonce,
        bytes: flat_account_proof_sized_bytes,
        data: flat_account_proof,
    };

    // ASSUMPTION: we are processing only one storage_key
    let eth_storage_proof = &eth_trie_proofs.storage_proof.first();
    if eth_storage_proof.is_none() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "storageProof is empty",
        )));
    }
    let eth_storage_proof = eth_storage_proof.unwrap();

    let storage_proof_ints_sequence: Vec<IntsSequence> = eth_storage_proof
        .proof
        .iter()
        .map(|element| -> IntsSequence { Data::from(HexString::new(element)).into() })
        .collect();

    let mut flat_storage_proof: Vec<u64> = Vec::new();
    let mut flat_storage_proof_sized_bytes = Vec::new();
    let mut flat_storage_proof_sized_words = Vec::new();

    for proof_element in storage_proof_ints_sequence.into_iter() {
        let words_length = proof_element.values.len();
        flat_storage_proof.extend(proof_element.values);
        flat_storage_proof_sized_bytes.push(proof_element.length);
        flat_storage_proof_sized_words.push(words_length);
    }

    let storage_proof = StorageProof {
        key: eth_storage_proof.key,
        value: eth_storage_proof.value,
        bytes: flat_storage_proof_sized_bytes,
        data: flat_storage_proof,
    };

    Ok(Proof {
        account_proof,
        storage_proof,
        len_proof,
        state_root: state_root.to_string(),
    })
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Proof {
    account_proof: AccountProof,
    storage_proof: StorageProof,
    len_proof: usize,
    state_root: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_proof() {
        let trie_proof = serde_json::json!({
        "accountProof": [
            "0xf90211a0849a7d0010e66b38686797062d027186829c413ccc53ebc128eedba8a4d2ce1fa078cf7ba19e7b93240969bcc080ea99a4506d2120c7ede76c0acb5933c03deeeca0376a01c4f6eff47aab45b20e25f16b4a0881ed3509af8de25d0c5755354c84fca033270a54a806cb7a90d725e79947452352cbe95df4ca652627f036bcce94a942a08b1df74630d969edf9afd27ad10996a9400451dd784fc9845d5f338382265f3ca00513b0ffacbf6450eb3189e2de3d0ee0cf8ee89738b696ecb64cf214d55e852da0ceb8d3435cfd934c2fa5a553083d45ae6ec6929e6ddf2b5fdf1efe71f84128aca0929275b6ae661f907de798e782451fca0551b79f357191f47afd94e2eea667b2a05c7341d98d0d086f00b61dfd7a2afbc8021a68db063eaa326e11ff94953a5d8ea07677075bae23ef69be265053189d55c79f84765d6ab7b7f3763761965774d9e6a0a136b035278a02200e78c741522727c2ef6609608bc4ab7653aa3858a0338c82a06e981991bde480df19151f36c27b6b73e8e19e31fcc123691c8bfadfdd77fd5da0e6e690cf243e0583f1be2a5a57e5b9c38c05e7eaec6655eb83c8415d5a3de081a0e337dddcb5b258a3a76fe202acf1b843f1f2a1b1853223198245d69748c55312a06508ee9aa79118b2708ca71c577eca7bde9509add600646351e44c4b5ef4163ea0e982a5900adb1fcbbaa984de41b330444a75e253b845782e9bbb1d7c414c39e080",
            "0xf90211a01ab53c74c38f15bedadf994101c54f876a13868bc8bd04f3cf965ba960de3b99a0272440853fbd481e5a84644cbdc6f9bc6b768051bbc0570d76d1a4b8469ad183a09667d6f0e95333862cf929a9c2502f7817c7a532cd509d3cfdf321c956534dcea0e841c36ab18b4894464c5a35e866e659d31e54796760cd80b062ef5e9a83dbd3a0ad44f6d8bc7a4c025c73496284595e63fc73e8c5ff57fca5321b5d1516ff930fa0bd114e2603542685db4e0d30318696875ed043db1359df415a7a0402432005a8a0efb310b5120b088eea878334ebe14d5f60bf430f45d06715feff49da4da6f5efa0a5b658d3e7467e58419b7a0211fd0fe3b407c49fb6996e82732582f6e659954aa011a2b0935001431fc24541c78ec9313d606bb34f6a8643447db262e180f95e3aa0ff1663789e8294e39a7e8d1820324bd1aaa4b8812451c5279ce7ca00b8120318a094de7b59762e4946bcaac99206ba4f5f3c498844fd8b3606c765f187306dcc10a049821564d5fa0726d4700550d4c339891d63c700169e1ac769b7f0c27e5e337da0c14e99657e6244b62e20c016bb0ad2454e4390ef8126bc4a9428a04d16e1fcb2a0cb56814f987997b030228f8698b3e0360586edcbfa63c0b5cacc9b6309f963f3a0f6e4744d20b8fdd5d883ec068b75f3000bc17136cc39cadca5d0372bfa276467a026693da8e92035c7538587777ed43c3b3750ff82fe90eeb2eb0aefb641d8186c80",
            "0xf90211a07f0993cf32869805d914fcb426bc0128c78adfc80c12b830077433d57c02298ea072fdabb94ca3953dd276a6b27885abe2751571cb134e91437fac3eeea0c44b42a0754a4f44aaa069690b2ac981c126231ee8d984192303829f60a29db2a0998c13a06277051098b5f69276d98a67b9f6814effe5dd6452fd406bb792da127b76722fa0ab7ef2cc4391b6626fcfe4577427550f88322ea4ff7808b0303399cc7cd81ff5a03fe34ab372e57196691e29ad9fe7aacd11c0862da431dc7215bbfaa2a7790aa5a0a5c91fe4c8993ad033843cfad2e412fac07142b0f4d1cdbbcfb87830ecb0a817a0fa48b2596e190c84a0656844421f3eff5b94061daa7a63dc324c39b8c49674d0a04d240e1d6c6a95401a783d8e06a075066d00d2f88e6f185a981bc01832010ecaa0f07b94d0b97716f092d02189885f21e877e65a88c1ff2d2ab2e245c7e33b06c1a06da79b83561b6732d7912a08fee482e6ace98bb870053d8e1b2427517d389fcfa0fafaa4902c40c9cd37f822a06615003784a1d602b96a436346227dad3a633310a019247a57a503e723f461061ff899ee751bc2e16ea2f77312e38fa23cca282890a0c68535c3ac930dd875360eca3470c02cdb0ba4b5dad4dd0d66e31f046bb3f710a0d27586f6ae5f11c671d9d147c58d1d611895a3defef30ca62b072d82ac79a959a09d23685d8e224a03070d6ea9fb2ff0af31dc38737a2b481ee9a2866aecc55c7280",
            "0xf90211a0ed3c028e567a3cc06ff93a341de07eb97d3bf363e708080dbede3fd535571dc3a0f188375a6a2dd9c3010a5fe1a8f221ebcfbc582e8c9d9a17226dcf5a3a238524a08c4475570a169d67e1f88d73edb95eddb01cc357839a9a1b84a9d73acc53da2fa047c3d161b5b97ce3e5cb7f937417d68bded846ffc963af5294a2ac5d555e859ca081cf83e5c33b1f88e62338531809f34f04072bbef4623c9915b99a471cf7ae99a067744ccf5ef6ac3c9c98cf33409e2a8799593fa878e8dec1f89590a1dcdb0257a0d9c4912fbbe5f355f7aedfb6bba239771a0fa3ebd24f2214665a389842c74c6aa0ca54465a547a0cf40034b13f0bac9da03de158c3152c09449dd4f5748dc8337ca009a0cc02081df35bcea63b03b6f4d0ce4eabb6f6fd91cd5bd68e856b51e037efa0374d3ca1194b9fe4ae44ca6f5f99c65ee45383355396461206ce07dfd5c9ad60a0db60afb2fc0bf6c274d1222f75008fd404bd0518b26152877ada3ab82526eb05a0c5024929c52154617804e024874b7cb07f83675c7fb4cb2f55afd098adcbe83da0318511ea3a37dd918fe3112349d47b40978e5d47c82ec61a31d2fdf605523873a0c58c09c6e6ab5e14f066d6ae307e64f72408f62c30d162d3a8b04ed6b9c644d9a07e8d525a53b6bb12c7d3a6945694e427974847f2ce90f0a3186ea28b1af1400ea0d715f517f7ad7d1ef1948ef106edfe623489d38d267adaa7719d2e72c06ff1d780",
            "0xf90211a0d3f3fabced08d104c9c3143e1837e89125e625f8446846682d3f6f24dac6f49aa0245f97ad6b03a76943c3c49b5b9f6ea7f713fdfbb8c816223bc61e076c958fcba005a99369aebbd8e41832f5c94cf77e699b51078b9d1902c445403ad8eb13afaaa06541f6fecc089483fad230199ef88fb128440704b9eaed5fdb0d6fcbf38525aea0e5c7650f712909187ba9e82f0bd7e25d08568d8cc1e7cfa8cb29f8d45d291f04a0358764934dffce82be9f3a9c056ded8b6e67353d05d017ae1668bda2b5060d5fa0bcc58f52a04c645587dccfc87f1c7d3e4031899c02501dddb0b43464e98892e5a0a1d60ce72bb4810c6d4119a15b3c33c2c675def6339942a7b30a92135f14915fa0dd67aa327f096131516b8bb29233a93eb0d5bd855f23ab73dd6805f48efc807ba0753d4f821a061eb60ee73aeb2cd809afdb32f556eb4280163088f0be9c756b40a0be3bdee948a810894269d55a5bf1ca2e04885e55bd24593aeda83d6dc93125d6a0bcd8795c0f1a54c18178a50490156ce0e37acd74a873819f0f8dbca019605183a0d99c3250fdb75bee213232c94d9774e5c65eab1c94e6a718e93a88ee0362e0a8a00ca7288fe5a5a8eff7194dbeb64a5f15356c02b5071e8afad01d46c6e66c890fa05843c2b0ee571052aebcb000e8aad739e30765e5cb1fecbb29b49584d3cefa60a0ac2c97fee3a54eb375a334e6a594e5412d82eec5ea7a2caaed699c3850369a0680",
            "0xf90211a0007dd7dbd837aed139221f449a71d0b4d8e8d496b62f515192f89377b7a54ce8a0816a701e02bc1484d167691ec621e91e3414034cec908cc400545a318a464b93a04eda27504c75434d02b26ff6f9cad4e7d9d85ce547bcfa9fb8a378b3c8104f2ca03bb6d6053b6075b7052c729a4ed73886abb90dae9609fcaf53caa1a2bd7fe9dca02a13f5b4a8f3c96922599f11713be6087a9fc7d7b715d2e42fe46c915ea4279fa0bd311e57cc8d62cbade777276d4b9d6a6a687ee6ef5d9c80c8280eb7d398d3c4a0f26e7856776c59abba1af1d80ebfde451f495342bcd4fedead55cc64cbb1fbf5a0d0cd9d93b6b192d87f66c43a758ea0dc9b7f75f26aeaef0466768cfa85be9f60a0b78a04c9a3f93c367e060ad7cf1b8dba2acda2aa525a56a064d1cb37d83c7a4aa0994446f7df83c740cd40444d2c18106d6ccdc36dcc99f4fa30b92be7661046eda08c63918e11055075450df554f632c2f009f8d9392595d5b52424bc2a87516734a045a9142bf2ebb004af147eb1db5ee2a569c24c2752dad628c521df69a484bc33a0c6785918addcd45644348c16b8fc9fa6d429d6f395d5c393e8f38982b1262357a08d552812d68aac20d6ea9641247ce9b0c7811416f7d8d25163eb8b8e2deb9f72a0233f7d0b62f66ebad765da6f63a284e9c1a519dfb17cd50c8d85c87d12dafaf1a0212ae8f250f899a4b871a534548836a64ee93aae083d5e596a84cff533ceaa5a80",
            "0xf8f180a0d7d633a58946d00ec7a1f93c1ca631df0a8673eae96370b6b72631a2907dfb77a020109805f7e2c937b8f329a424fecbb2a67ec3ccd55b48f7ead0dcca74c73d6480a0b40cabe387ab1ae0428e6acd4f45febed642e349e6ff78e5a0c05562312e524fa08ea99d2bb64a345ce2b71b68bca347bce6b8682a48398e889e78167d989067d58080a06da6a64497a321688bf2df91fadc67562ad61132cff63169753fccb53ead595e80a04145b159ba7e409b02abc554aa956278ef224a1b19afc7c6ffb435eb7585a59e8080a0aa7d188521de91c9d7605a4c7529e733f0a67d82f5fbab4c0a1435c8d4bd2afc808080",
            "0xf8669d338cfc997a82252167ac25a16580d9730353eb1b9f0c6bbf0e4c82c4d0b846f8440180a0199c2e6b850bcc9beaea25bf1bacc5741a7aad954d28af9b23f4b53f5404937ba04e36f96ee1667a663dfaac57c4d185a0e369a3a217e0079d49620f34f85d1ac7"
        ],
        "address": "0x6b175474e89094c44da98b954eedeac495271d0f",
        "balance": "0x0",
        "codeHash": "0x4e36f96ee1667a663dfaac57c4d185a0e369a3a217e0079d49620f34f85d1ac7",
        "nonce": "0x1",
        "storageHash": "0x199c2e6b850bcc9beaea25bf1bacc5741a7aad954d28af9b23f4b53f5404937b",
        "storageProof": [
            {
                "key": "0x492bf5562c9e94709c6e18b3f039db48d51df4ea96a4b9b83745deb2a29fbf0b",
                "proof": [
                    "0xf90211a0598782f8387587e1e8f6ae7350402b33c23fcd1ed40c29be87a33ae270d6cc09a030f353bc01f7cd885a027a236c9f315b6b404d9a97fa5b4b5f2994bc1c36a2c4a07513f383044878853dd84e982e63fa22be8be9e765b9a575d5be5ed6ef077e5da0fbba2a12f6b9ba375b89ba56d738f32b55c413c164f6659f657f0b22670f7d66a09ec5b89ac4129d65820a0927c420300ac8b96e62c62163522fc1e52fb2026d43a08f86fe52560ed54729d5b0bb1f2aec38602c49e15440124132a18b35aa310bfea05e204f715be9415abef91aca0cf6446ecbfda6319b31d2881f4333a07fc08b6ba0915d216a6ffa3490427ac52278f8d3005cf194cf76b44bb9b470f604a3ea2478a02975cb05c8672dce0bc03190338f8b557783215a18ae11308dfa5e553cb4f6efa0b8b1a9242e569f669a6c16481fff329ccecf2885ddbe21979b01361d7d1d49d3a05619aad7f99c9f5afe6fa6fb6e45ce2adcc0d172c1b79ce2cd5a2546c409821ea04cbb6a4a06bc465b0d6073469b5037265156d1ae712ddf4c8b7978db71944ddea031329613f73a08e57c079bee5c4fb07a41fa6a973331e4023c9d4605ec035351a0c42534bf3b026bf0ab03938181ef8e06c53ed0060fc52626267d8bb1b11beab1a064a1f6dc1b14100509d87e14399c3e5e3e2ed34c06dac6aeda67e9d105830340a004ab0120589ac2662aa84edc9d40907a446836c28ac28aec40d4c7321ef5d70080",
                    "0xf90211a011d28387d528a56e707fa0187d0d3dd18054dd4bd0de2141ef04f4bc76c5cbf4a0757770e5bb7dabc87efb94fc1e9e2f1566f17e31dcdd2e51d8b960d2b2ce16a6a037110a7ef6ba4010fe4d41002026298bce084d35d746e21cfd932cd7d58e3d06a0626542b2b0a4d680cc9c5bdae8fc87abb197a426587d09fb8cc3acb871bb4a65a000cd02293e248f8c25c77608685d051b79f577c38b12872a5a5b430ce97bb560a0c5a544efcb5021d0e1ba2a7f810d3c128c86b4a6ed7950fcf1d63845cad95294a031298f76fa10cbd8f8ec69d2d7ed67369fe9073f39e451b1553f39ead2a0793aa019698f02f865a2a978468c42db69ec3438fc606cb5366f36fd4a870b2e2fc064a08b200e6d9214d3bd239c3646a77a883ec105f60ff17af64634ff3adad5cae2f1a0357948cd283c25f3a7841b8473142187d17456e6b21ad97ae8e1bcc8e45437b2a044a0b0ead881a3b6a9042612cfd77c75710dc926a85b85df1a95848926cc572aa09bcf52ca4735113398c7391ef4e6ec4f184df286f992619ca3e48b00dbdbdc7ba0f664cb902efa821998e3d0c97c7a95aca817df66a2404d25eed3d2fec5163c85a07aaf7e99d5633edbebbdef212617ec541a51a780ebadfec5ad51b97695bab2f4a0c1cf30942b46bc6d2d779b0cbf51c4cf7e8bddf0605c199cffe89012e9346ae0a0e631be5028bebd4be1bd52c4cc8bbefa650cabd0a012030ada1d973afe6c199180",
                    "0xf90211a048710c80f3d857b3b8ef7d06a3a1220ec0125bb5c7e9b138c8fca7ed12d8555fa0465eba00147c1c4abc085e3bcb3e8ec831269927de21e732a2b5f516765795a2a059e392895f9baeb187f263fa6ded49cf2fd6e05bc54480971ebfb5e80ad28617a0a3380fa80d02aad22611dcd570111033170223da9f52be0c653c88a249fcdb11a06a5e6c5fd023804fb9457a5ea13bdcd92225f5629e27b0d3f428de783a2a4a72a05a3e5ed7a9e4fefcbe2be01a0582e22d3d60b692ee4e3fd16ed6059626724f70a0faf59281fa0b05fefc1cf132df0634b522a8a7b20eca0c86e14147662464e548a0251cade67855e9ef19f079cca521a8267727171a1e13728543a477bd5fcf4a1fa0dd0e6b6ab78d30fea3735938b96ab883fa0c3e8fdbdb5d85371c45d18d69486aa0eb212dbd6e480526501a97a75d5dc3739e63349b23083b1460047eaaed694f80a0165dfe8c92c29b72baf429b44463aadee7f1a5bfaee7f34c0964811e00fe28aea04961d760d7e33682bfbd3a590905952caecca9f5fc2dbf45ed00daa538e2bb24a002b5ad20bf0bbef8334836807ec49a31c8097098226377cf16ef9f11252e71a7a04cc8c0ad8565a57fc1c38db083bd9be9dada78cc5637761c4f22bedd54478ce6a08c6326486006d77a97f3a1774f63e3f7560975d8b7eda1a25cf471577f76a050a0f2756a0dbdd880acd471093d367eca8c41c51be210dad821ee4a3c5110350d2680",
                    "0xf90211a0cd6a6a94dd8a981e6b629d33caa2560d3f3e8c2e882efd0983190035bf2fec5da0269024ddb79b84d61c828ef91378db51bc7d817f1f9603eabbf32b770fb1b08ca055f640dc3755c6006a94cb9807aff8aac11f0ce19862083238acbf6df3e01173a0cc238a3015a8a82afc4bec75c88b650161e4de48c068bac3e837d3ba21ee2eada071d8de06fe4c7e219f7af19878599d93cb56d1161a9fe64c3e48b09f5e2065f0a00cc2dca28be3cc0e6dc87eed8ded6339450722573d51303474aa7234dec393aea0c289cca51cfbc82af51948cc64f292109978a2ac020770b1cc8c42426a496f06a0b6fa2afd5bc01fbe5da0f342df376c413e3e1c25563f9fd72a98832c28c96cd3a026034fa84385dae4c484a3ffb538756af14b171f7d0c5d8bf7acdfe59f751df4a033f6bfdacdd9b457979d63a15c4118c7c25845a2bf09bb751d8f5d93410ecfeaa05cf9c966507a8e57626123f9ac15cd7ccc79abbff9773f0e0c046e5f6dd3f17aa09e653aaa9b0d972f237f58d1ffb146f7a9b4ead72113e44b1e1194f06dc073f1a0fd724b9c51e08718b13d9aae262cb2de60100d9d6569ce3569de4dbf10de17e6a00055d032b0444d9d9dabc905ed8b14e18f932bba31329ca6131bfea41f4d9171a0b6db7905e8d67cd245c5edde8db9f76973394b59c6be0f9a1f1092acfa3a40e3a0654584c1cc39d4dd2eec590d13fef1651e2ef2fccaebc0c230aa1776197de93780",
                    "0xf901d1a068a4d07a0e90ed019317244f7d575f6e2d030955c8d65243026393c7317f985580a0708e0dfa1878f195c0ca96be333cf741fce31d18d3fe458c5c1bd13d82330ffaa0e728afa398c66ba67526561731609cf03ae8ed112d6f7f43a84ad0966f277cb3a0205c4beb6b1b674eadc8c6b46bbb39a15c25371cc60e0dab01affa4ca1899156a052b9439a85426df65d20c57ccccb6d13fa0e88b354f514908163574870fbec3580a0b244fa6403784424be652baec0dccc9f54f7eca0aa5ed36f8c0460742cfae059a0cab7cabb3a32db5f4bfbc62861659187e56210beb9d140b99b98af773ea7256ea0d2e914a8594d0c7d244ba53f615e74dc096c64eecd2d1cce4c85f8ed57478e22a03afffd0233bd2dc4e22fa733e7f7bffb6aa0455f180e00d6def07b2c336a3009a0a9f5faead6cc7979361dcd0638070ad42e4a514cc80b27214050a91836cd44f3a04116ba03b01f2c14a4da08450973bd1250c97e95904c2222b2df81eef30ea1caa0e8c8dc48a5f360231d6f7df14cecd635b23281c422c8c2b7c7e4ae9a092c879da0803e2d872008805fba967f38254dc7891bdc92886353d1aa997c1020bf27c5e4a081eaf30df534be04fce4dd75e79b6a53da8bcf035bb322f5d0ead8c69e42643880",
                    "0xf8518080808080a0f1ad59399f0c30f74ce5df49751c9678149ebb4ccbc90a2a7a077b48ae096594a00b77d89e9292ff2a6e3ae251ad3eaeda458946090db7efc03db8ede8f3aafc8380808080808080808080",
                    "0xeb9e207e86b2c52e66ce3aa563e794cfce226abfd7ce86ad441925e3740f64e98b8a07a8f1298f7eaa479401"
                ],
                "value": "0x07a8f1298f7eaa479401"
            }
        ]
        });

        let storage_keys =
            vec!["0x2045bf4ea5561e88a4d0d9afbc316354e49fe892ac7e961a5e68f1f4b9561152".to_owned()];

        let res = calculate_proof(&trie_proof, storage_keys).unwrap();

        assert_eq!(
            res.storage_proof.data,
            vec![
                17942923245791970040,
                4068307242744983155,
                5782669422846004510,
                15279633534057396962,
                8130910518978343763,
                13547381616235119226,
                2552590123148197965,
                11139647473337706900,
                13554769051189802259,
                17546873282471738840,
                5663327538351619723,
                16854551978476688830,
                6833912300045181179,
                13414555290963687259,
                9924340211237727061,
                14128849095366123365,
                9154448493597517472,
                11440753403258641765,
                9370312041096556554,
                14463713048786985810,
                3441283583300496707,
                11569614394599280341,
                5127864605768362732,
                4062295559712817170,
                4697994980573720843,
                18347768374772194281,
                4709286337474858230,
                4931102932292705073,
                15170409718286942144,
                10046299939973196399,
                18029193823472919160,
                17929674940682129270,
                12991681734720750755,
                16871742733322406661,
                14440561093170442640,
                3715341416797512026,
                1778377803096022613,
                4374392647137669545,
                2607116476915280918,
                5197153087853088552,
                9646075028384186678,
                2124886802109060633,
                12310582557524295279,
                12032332075541126336,
                15092338295627238746,
                2686049772889874508,
                13504687825412840205,
                6949976282842474065,
                6255973158518475915,
                8752987156660477600,
                3545060869190322405,
                8937283433593286778,
                4754229554274493442,
                4367724204867670865,
                11584425049980600939,
                17341958722188078990,
                487865197815383334,
                2749022661491366890,
                12799340787781081876,
                1154339704923240860,
                4494097848996726490,
                14316620405296661891,
                234363122532556888,
                11151587960721169565,
                4652352849298506378,
                14018276553335976478,
                4124508288,
                17942923244588925831,
                15359708422899474456,
                9010926399448735051,
                15050503571785446588,
                8558470918412007280,
                16553963032087034772,
                18167131871918879102,
                3592989793746205024,
                15182423890499483409,
                756313067244027469,
                4683778960743190024,
                5563589614127349139,
                3231286064547536994,
                7296590818043134156,
                11266839587001445297,
                10926900757404449676,
                14099847332003210656,
                57704746385182604,
                2722274278251496731,
                8788061929618966314,
                6510871409030051168,
                11584847331979907105,
                15051516121339465020,
                1336591299875010896,
                18226584704122542418,
                10709613968457398800,
                14688763878940268525,
                7437307657846733284,
                5886579917992940192,
                8735470458244104952,
                7323602278102614747,
                7632532887843138741,
                3922414261967981358,
                3440860756002737773,
                10526271038371411526,
                12068107954600408591,
                17400490891696618202,
                15405375002777057608,
                14783131910542951451,
                9543994169573340246,
                16623378695950229948,
                14475787700472399008,
                12748239746534648068,
                2743483647435043085,
                14494457561780001429,
                9550207143893442715,
                14939225321123165080,
                14355539324415266584,
                5616700091745279139,
                16468257406696848288,
                17754539451125236249,
                11016878677273777580,
                12112395354738019621,
                17209330592782892165,
                11563747951508808510,
                15846968548584200172,
                6060246028444741118,
                14244131053443070642,
                17627302037145529158,
                13577558443317575505,
                14181692890802053212,
                1845631321629714740,
                7701332273341026344,
                13744225070720664780,
                10069761128758235296,
                1297893049528433406,
                1813614976,
                17942923245505285248,
                17570890375599455494,
                11790742746179984309,
                14405239740569528301,
                1357929156851424954,
                5765960488978526,
                4308606251474953881,
                2872770889179641333,
                1618577416276433379,
                10559075923602278386,
                7204191419845521366,
                16166732185382362815,
                13107738614770933923,
                4039632164607414822,
                1287138270820446999,
                154207190083570789,
                4361914677766853024,
                7664682774600843343,
                13350211217260272857,
                2460642575336845523,
                17593556552276134514,
                11554616371038512382,
                18212042184642101986,
                3259868042551512639,
                15091234720847393359,
                8115762461153557003,
                432059815510073094,
                3797979968520195786,
                902656445821363300,
                16521631314264057464,
                6190742057485192357,
                2425230692537801246,
                1401328959634259295,
                14936785889891609450,
                13226281550879086904,
                13360694121523330703,
                15842358940942484945,
                10189755254705955117,
                13649926707561372311,
                12059898471207822132,
                11178787749901632638,
                12316616346109482589,
                18342196746902485748,
                3005102046222149617,
                11943457048052369764,
                9303874971798249545,
                7050210222018036415,
                13635308717100838062,
                14747588918249473517,
                61543208663590048,
                195252515016261368,
                3695263419768019505,
                14414175881090267087,
                1652714484255322535,
                11550827874168956325,
                9205854126641233307,
                16851021168119265142,
                2039887359374542732,
                16618436982469058566,
                15526889737955004259,
                16426692764489005037,
                11646973791276859254,
                11551910007417736637,
                15600659137640873270,
                9136269008182174224,
                15769391402569191696,
                890054272,
                17942923247736220308,
                15963739085586210099,
                14601327556784196654,
                9813058855382876213,
                13776489672012632100,
                15976409197950304910,
                17947821823366036865,
                9160205110662132523,
                8579271186995762678,
                4673671354837396116,
                14670484238532002079,
                928190545026758828,
                13793949377396711628,
                2560912207695588092,
                5470877151540871521,
                16491698876705326056,
                4022763547064708512,
                8203550842872954401,
                11491763036345703827,
                14652128330562659916,
                4488031227200169456,
                11532806998664668108,
                1039707536469388643,
                4126712477628846384,
                3779833394760696723,
                12583271255972846843,
                14423610245551777010,
                10525081072569156103,
                8120496505904851529,
                8000258495811878235,
                13843993088901726943,
                3993638705156597078,
                4584619523633785896,
                14514208383894245288,
                4865535648528114687,
                13058316221735966495,
                9010679810198659045,
                11490122960802674367,
                15766497239080869219,
                11627239912650201157,
                11727102655424466781,
                10610778483218996473,
                14512375336552325729,
                2592292270170229881,
                12375884591591721988,
                7953196224016457886,
                7294330129404604195,
                9176315136888469417,
                13036468608444943134,
                1266901749368877472,
                18262742573701433112,
                12771534191048045278,
                6922047596930911797,
                7628620301829281766,
                11529309398760440909,
                11357422715174488852,
                16253371396220072604,
                11966939415342697873,
                8187745176304085206,
                8994328122213961145,
                17827907289837323966,
                1124245213023107642,
                4675757194959176140,
                4167198748828044563,
                18370575535332064458,
                16987791308709393945,
                2112436096,
                17942852877301371002,
                1049599304195843151,
                9031792504521034069,
                14471844900358034375,
                3566736922898362510,
                1007144374398992586,
                10862175688203566307,
                2096658715582946331,
                15077350283329315047,
                2931741759000782453,
                2762420922280112186,
                16784090273121518504,
                5390974158110503840,
                2331822181557430094,
                12522477242107509153,
                6639773822349086123,
                121590921996898646,
                11552499694165115501,
                17752381338738805613,
                1439479011185325332,
                10412713039985966060,
                3855257968551027715,
                8666091983127424704,
                15910266770878668970,
                6832927706810709036,
                18077547451395852987,
                4193655405378520616,
                7018175607181807806,
                13389554333072797559,
                4514618308537018644,
                12130811787333618597,
                4567035353131215972,
                17207459352995988984,
                17102216785754602239,
                18231191129158902319,
                12048228479410072224,
                4998740559827230448,
                8875525496805892265,
                17724737491744422198,
                2147379435031680046,
                5355145853984514368,
                5812203417749484448,
                4690140586955451412,
                11878816059747450130,
                5821323174189212194,
                12889163521837670858,
                11594738089102537568,
                2530301152362032342,
                3869210562787461314,
                13242804667865967751,
                11358219264768548872,
                9250317315184534861,
                14378053919522186067,
                15108056658093326119,
                14259698800014855669,
                3800480619765265895,
                11198855621523735387,
                12908149759639340702,
                1113864320,
                17893224083919765664,
                17414673438127960311,
                5541080423139677816,
                1485830865589701162,
                8793132349551895956,
                11532443043069334271,
                3057445940738473646,
                15728128207553935343,
                13852431360816556796,
                9475714905387597952,
                8421504,
                16978043373031179566,
                7407922919091180751,
                14853551893213251245,
                4906994927831835881,
                10054857540239986558,
                2856817665
            ]
        );
        assert_eq!(
            res.storage_proof.bytes,
            vec![532, 532, 532, 532, 468, 83, 44]
        );
        assert_eq!(res.len_proof, 446);
        assert_eq!(
            res.account_proof.address,
            U256::from_dec_str("611382286831621467233887798921843936019654057231").unwrap(),
        );
        assert_eq!(
            res.state_root,
            "14597243955974265521283702380833262217555605356628210962737576498440829276498"
        );
    }

    #[test]
    fn account_proof_empty() {
        let trie_proof = serde_json::json!({
        "accountProof": [
        ],
        "address": "0x6b175474e89094c44da98b954eedeac495271d0f",
        "balance": "0x0",
        "codeHash": "0x4e36f96ee1667a663dfaac57c4d185a0e369a3a217e0079d49620f34f85d1ac7",
        "nonce": "0x1",
        "storageHash": "0x199c2e6b850bcc9beaea25bf1bacc5741a7aad954d28af9b23f4b53f5404937b",
        "storageProof": [
            {
                "key": "0x492bf5562c9e94709c6e18b3f039db48d51df4ea96a4b9b83745deb2a29fbf0b",
                "proof": [
                    "0xf90211a0598782f8387587e1e8f6ae7350402b33c23fcd1ed40c29be87a33ae270d6cc09a030f353bc01f7cd885a027a236c9f315b6b404d9a97fa5b4b5f2994bc1c36a2c4a07513f383044878853dd84e982e63fa22be8be9e765b9a575d5be5ed6ef077e5da0fbba2a12f6b9ba375b89ba56d738f32b55c413c164f6659f657f0b22670f7d66a09ec5b89ac4129d65820a0927c420300ac8b96e62c62163522fc1e52fb2026d43a08f86fe52560ed54729d5b0bb1f2aec38602c49e15440124132a18b35aa310bfea05e204f715be9415abef91aca0cf6446ecbfda6319b31d2881f4333a07fc08b6ba0915d216a6ffa3490427ac52278f8d3005cf194cf76b44bb9b470f604a3ea2478a02975cb05c8672dce0bc03190338f8b557783215a18ae11308dfa5e553cb4f6efa0b8b1a9242e569f669a6c16481fff329ccecf2885ddbe21979b01361d7d1d49d3a05619aad7f99c9f5afe6fa6fb6e45ce2adcc0d172c1b79ce2cd5a2546c409821ea04cbb6a4a06bc465b0d6073469b5037265156d1ae712ddf4c8b7978db71944ddea031329613f73a08e57c079bee5c4fb07a41fa6a973331e4023c9d4605ec035351a0c42534bf3b026bf0ab03938181ef8e06c53ed0060fc52626267d8bb1b11beab1a064a1f6dc1b14100509d87e14399c3e5e3e2ed34c06dac6aeda67e9d105830340a004ab0120589ac2662aa84edc9d40907a446836c28ac28aec40d4c7321ef5d70080",
                    "0xf90211a011d28387d528a56e707fa0187d0d3dd18054dd4bd0de2141ef04f4bc76c5cbf4a0757770e5bb7dabc87efb94fc1e9e2f1566f17e31dcdd2e51d8b960d2b2ce16a6a037110a7ef6ba4010fe4d41002026298bce084d35d746e21cfd932cd7d58e3d06a0626542b2b0a4d680cc9c5bdae8fc87abb197a426587d09fb8cc3acb871bb4a65a000cd02293e248f8c25c77608685d051b79f577c38b12872a5a5b430ce97bb560a0c5a544efcb5021d0e1ba2a7f810d3c128c86b4a6ed7950fcf1d63845cad95294a031298f76fa10cbd8f8ec69d2d7ed67369fe9073f39e451b1553f39ead2a0793aa019698f02f865a2a978468c42db69ec3438fc606cb5366f36fd4a870b2e2fc064a08b200e6d9214d3bd239c3646a77a883ec105f60ff17af64634ff3adad5cae2f1a0357948cd283c25f3a7841b8473142187d17456e6b21ad97ae8e1bcc8e45437b2a044a0b0ead881a3b6a9042612cfd77c75710dc926a85b85df1a95848926cc572aa09bcf52ca4735113398c7391ef4e6ec4f184df286f992619ca3e48b00dbdbdc7ba0f664cb902efa821998e3d0c97c7a95aca817df66a2404d25eed3d2fec5163c85a07aaf7e99d5633edbebbdef212617ec541a51a780ebadfec5ad51b97695bab2f4a0c1cf30942b46bc6d2d779b0cbf51c4cf7e8bddf0605c199cffe89012e9346ae0a0e631be5028bebd4be1bd52c4cc8bbefa650cabd0a012030ada1d973afe6c199180",
                    "0xf90211a048710c80f3d857b3b8ef7d06a3a1220ec0125bb5c7e9b138c8fca7ed12d8555fa0465eba00147c1c4abc085e3bcb3e8ec831269927de21e732a2b5f516765795a2a059e392895f9baeb187f263fa6ded49cf2fd6e05bc54480971ebfb5e80ad28617a0a3380fa80d02aad22611dcd570111033170223da9f52be0c653c88a249fcdb11a06a5e6c5fd023804fb9457a5ea13bdcd92225f5629e27b0d3f428de783a2a4a72a05a3e5ed7a9e4fefcbe2be01a0582e22d3d60b692ee4e3fd16ed6059626724f70a0faf59281fa0b05fefc1cf132df0634b522a8a7b20eca0c86e14147662464e548a0251cade67855e9ef19f079cca521a8267727171a1e13728543a477bd5fcf4a1fa0dd0e6b6ab78d30fea3735938b96ab883fa0c3e8fdbdb5d85371c45d18d69486aa0eb212dbd6e480526501a97a75d5dc3739e63349b23083b1460047eaaed694f80a0165dfe8c92c29b72baf429b44463aadee7f1a5bfaee7f34c0964811e00fe28aea04961d760d7e33682bfbd3a590905952caecca9f5fc2dbf45ed00daa538e2bb24a002b5ad20bf0bbef8334836807ec49a31c8097098226377cf16ef9f11252e71a7a04cc8c0ad8565a57fc1c38db083bd9be9dada78cc5637761c4f22bedd54478ce6a08c6326486006d77a97f3a1774f63e3f7560975d8b7eda1a25cf471577f76a050a0f2756a0dbdd880acd471093d367eca8c41c51be210dad821ee4a3c5110350d2680",
                    "0xf90211a0cd6a6a94dd8a981e6b629d33caa2560d3f3e8c2e882efd0983190035bf2fec5da0269024ddb79b84d61c828ef91378db51bc7d817f1f9603eabbf32b770fb1b08ca055f640dc3755c6006a94cb9807aff8aac11f0ce19862083238acbf6df3e01173a0cc238a3015a8a82afc4bec75c88b650161e4de48c068bac3e837d3ba21ee2eada071d8de06fe4c7e219f7af19878599d93cb56d1161a9fe64c3e48b09f5e2065f0a00cc2dca28be3cc0e6dc87eed8ded6339450722573d51303474aa7234dec393aea0c289cca51cfbc82af51948cc64f292109978a2ac020770b1cc8c42426a496f06a0b6fa2afd5bc01fbe5da0f342df376c413e3e1c25563f9fd72a98832c28c96cd3a026034fa84385dae4c484a3ffb538756af14b171f7d0c5d8bf7acdfe59f751df4a033f6bfdacdd9b457979d63a15c4118c7c25845a2bf09bb751d8f5d93410ecfeaa05cf9c966507a8e57626123f9ac15cd7ccc79abbff9773f0e0c046e5f6dd3f17aa09e653aaa9b0d972f237f58d1ffb146f7a9b4ead72113e44b1e1194f06dc073f1a0fd724b9c51e08718b13d9aae262cb2de60100d9d6569ce3569de4dbf10de17e6a00055d032b0444d9d9dabc905ed8b14e18f932bba31329ca6131bfea41f4d9171a0b6db7905e8d67cd245c5edde8db9f76973394b59c6be0f9a1f1092acfa3a40e3a0654584c1cc39d4dd2eec590d13fef1651e2ef2fccaebc0c230aa1776197de93780",
                    "0xf901d1a068a4d07a0e90ed019317244f7d575f6e2d030955c8d65243026393c7317f985580a0708e0dfa1878f195c0ca96be333cf741fce31d18d3fe458c5c1bd13d82330ffaa0e728afa398c66ba67526561731609cf03ae8ed112d6f7f43a84ad0966f277cb3a0205c4beb6b1b674eadc8c6b46bbb39a15c25371cc60e0dab01affa4ca1899156a052b9439a85426df65d20c57ccccb6d13fa0e88b354f514908163574870fbec3580a0b244fa6403784424be652baec0dccc9f54f7eca0aa5ed36f8c0460742cfae059a0cab7cabb3a32db5f4bfbc62861659187e56210beb9d140b99b98af773ea7256ea0d2e914a8594d0c7d244ba53f615e74dc096c64eecd2d1cce4c85f8ed57478e22a03afffd0233bd2dc4e22fa733e7f7bffb6aa0455f180e00d6def07b2c336a3009a0a9f5faead6cc7979361dcd0638070ad42e4a514cc80b27214050a91836cd44f3a04116ba03b01f2c14a4da08450973bd1250c97e95904c2222b2df81eef30ea1caa0e8c8dc48a5f360231d6f7df14cecd635b23281c422c8c2b7c7e4ae9a092c879da0803e2d872008805fba967f38254dc7891bdc92886353d1aa997c1020bf27c5e4a081eaf30df534be04fce4dd75e79b6a53da8bcf035bb322f5d0ead8c69e42643880",
                    "0xf8518080808080a0f1ad59399f0c30f74ce5df49751c9678149ebb4ccbc90a2a7a077b48ae096594a00b77d89e9292ff2a6e3ae251ad3eaeda458946090db7efc03db8ede8f3aafc8380808080808080808080",
                    "0xeb9e207e86b2c52e66ce3aa563e794cfce226abfd7ce86ad441925e3740f64e98b8a07a8f1298f7eaa479401"
                ],
                "value": "0x07a8f1298f7eaa479401"
            }
        ]
        });

        let storage_keys =
            vec!["0x2045bf4ea5561e88a4d0d9afbc316354e49fe892ac7e961a5e68f1f4b9561152".to_owned()];

        let res = calculate_proof(&trie_proof, storage_keys);

        assert!(res.is_err());
    }

    #[test]
    fn storage_proof_empty() {
        let trie_proof = serde_json::json!({
        "accountProof": [
            "0xf90211a0849a7d0010e66b38686797062d027186829c413ccc53ebc128eedba8a4d2ce1fa078cf7ba19e7b93240969bcc080ea99a4506d2120c7ede76c0acb5933c03deeeca0376a01c4f6eff47aab45b20e25f16b4a0881ed3509af8de25d0c5755354c84fca033270a54a806cb7a90d725e79947452352cbe95df4ca652627f036bcce94a942a08b1df74630d969edf9afd27ad10996a9400451dd784fc9845d5f338382265f3ca00513b0ffacbf6450eb3189e2de3d0ee0cf8ee89738b696ecb64cf214d55e852da0ceb8d3435cfd934c2fa5a553083d45ae6ec6929e6ddf2b5fdf1efe71f84128aca0929275b6ae661f907de798e782451fca0551b79f357191f47afd94e2eea667b2a05c7341d98d0d086f00b61dfd7a2afbc8021a68db063eaa326e11ff94953a5d8ea07677075bae23ef69be265053189d55c79f84765d6ab7b7f3763761965774d9e6a0a136b035278a02200e78c741522727c2ef6609608bc4ab7653aa3858a0338c82a06e981991bde480df19151f36c27b6b73e8e19e31fcc123691c8bfadfdd77fd5da0e6e690cf243e0583f1be2a5a57e5b9c38c05e7eaec6655eb83c8415d5a3de081a0e337dddcb5b258a3a76fe202acf1b843f1f2a1b1853223198245d69748c55312a06508ee9aa79118b2708ca71c577eca7bde9509add600646351e44c4b5ef4163ea0e982a5900adb1fcbbaa984de41b330444a75e253b845782e9bbb1d7c414c39e080",
            "0xf90211a01ab53c74c38f15bedadf994101c54f876a13868bc8bd04f3cf965ba960de3b99a0272440853fbd481e5a84644cbdc6f9bc6b768051bbc0570d76d1a4b8469ad183a09667d6f0e95333862cf929a9c2502f7817c7a532cd509d3cfdf321c956534dcea0e841c36ab18b4894464c5a35e866e659d31e54796760cd80b062ef5e9a83dbd3a0ad44f6d8bc7a4c025c73496284595e63fc73e8c5ff57fca5321b5d1516ff930fa0bd114e2603542685db4e0d30318696875ed043db1359df415a7a0402432005a8a0efb310b5120b088eea878334ebe14d5f60bf430f45d06715feff49da4da6f5efa0a5b658d3e7467e58419b7a0211fd0fe3b407c49fb6996e82732582f6e659954aa011a2b0935001431fc24541c78ec9313d606bb34f6a8643447db262e180f95e3aa0ff1663789e8294e39a7e8d1820324bd1aaa4b8812451c5279ce7ca00b8120318a094de7b59762e4946bcaac99206ba4f5f3c498844fd8b3606c765f187306dcc10a049821564d5fa0726d4700550d4c339891d63c700169e1ac769b7f0c27e5e337da0c14e99657e6244b62e20c016bb0ad2454e4390ef8126bc4a9428a04d16e1fcb2a0cb56814f987997b030228f8698b3e0360586edcbfa63c0b5cacc9b6309f963f3a0f6e4744d20b8fdd5d883ec068b75f3000bc17136cc39cadca5d0372bfa276467a026693da8e92035c7538587777ed43c3b3750ff82fe90eeb2eb0aefb641d8186c80",
            "0xf90211a07f0993cf32869805d914fcb426bc0128c78adfc80c12b830077433d57c02298ea072fdabb94ca3953dd276a6b27885abe2751571cb134e91437fac3eeea0c44b42a0754a4f44aaa069690b2ac981c126231ee8d984192303829f60a29db2a0998c13a06277051098b5f69276d98a67b9f6814effe5dd6452fd406bb792da127b76722fa0ab7ef2cc4391b6626fcfe4577427550f88322ea4ff7808b0303399cc7cd81ff5a03fe34ab372e57196691e29ad9fe7aacd11c0862da431dc7215bbfaa2a7790aa5a0a5c91fe4c8993ad033843cfad2e412fac07142b0f4d1cdbbcfb87830ecb0a817a0fa48b2596e190c84a0656844421f3eff5b94061daa7a63dc324c39b8c49674d0a04d240e1d6c6a95401a783d8e06a075066d00d2f88e6f185a981bc01832010ecaa0f07b94d0b97716f092d02189885f21e877e65a88c1ff2d2ab2e245c7e33b06c1a06da79b83561b6732d7912a08fee482e6ace98bb870053d8e1b2427517d389fcfa0fafaa4902c40c9cd37f822a06615003784a1d602b96a436346227dad3a633310a019247a57a503e723f461061ff899ee751bc2e16ea2f77312e38fa23cca282890a0c68535c3ac930dd875360eca3470c02cdb0ba4b5dad4dd0d66e31f046bb3f710a0d27586f6ae5f11c671d9d147c58d1d611895a3defef30ca62b072d82ac79a959a09d23685d8e224a03070d6ea9fb2ff0af31dc38737a2b481ee9a2866aecc55c7280",
            "0xf90211a0ed3c028e567a3cc06ff93a341de07eb97d3bf363e708080dbede3fd535571dc3a0f188375a6a2dd9c3010a5fe1a8f221ebcfbc582e8c9d9a17226dcf5a3a238524a08c4475570a169d67e1f88d73edb95eddb01cc357839a9a1b84a9d73acc53da2fa047c3d161b5b97ce3e5cb7f937417d68bded846ffc963af5294a2ac5d555e859ca081cf83e5c33b1f88e62338531809f34f04072bbef4623c9915b99a471cf7ae99a067744ccf5ef6ac3c9c98cf33409e2a8799593fa878e8dec1f89590a1dcdb0257a0d9c4912fbbe5f355f7aedfb6bba239771a0fa3ebd24f2214665a389842c74c6aa0ca54465a547a0cf40034b13f0bac9da03de158c3152c09449dd4f5748dc8337ca009a0cc02081df35bcea63b03b6f4d0ce4eabb6f6fd91cd5bd68e856b51e037efa0374d3ca1194b9fe4ae44ca6f5f99c65ee45383355396461206ce07dfd5c9ad60a0db60afb2fc0bf6c274d1222f75008fd404bd0518b26152877ada3ab82526eb05a0c5024929c52154617804e024874b7cb07f83675c7fb4cb2f55afd098adcbe83da0318511ea3a37dd918fe3112349d47b40978e5d47c82ec61a31d2fdf605523873a0c58c09c6e6ab5e14f066d6ae307e64f72408f62c30d162d3a8b04ed6b9c644d9a07e8d525a53b6bb12c7d3a6945694e427974847f2ce90f0a3186ea28b1af1400ea0d715f517f7ad7d1ef1948ef106edfe623489d38d267adaa7719d2e72c06ff1d780",
            "0xf90211a0d3f3fabced08d104c9c3143e1837e89125e625f8446846682d3f6f24dac6f49aa0245f97ad6b03a76943c3c49b5b9f6ea7f713fdfbb8c816223bc61e076c958fcba005a99369aebbd8e41832f5c94cf77e699b51078b9d1902c445403ad8eb13afaaa06541f6fecc089483fad230199ef88fb128440704b9eaed5fdb0d6fcbf38525aea0e5c7650f712909187ba9e82f0bd7e25d08568d8cc1e7cfa8cb29f8d45d291f04a0358764934dffce82be9f3a9c056ded8b6e67353d05d017ae1668bda2b5060d5fa0bcc58f52a04c645587dccfc87f1c7d3e4031899c02501dddb0b43464e98892e5a0a1d60ce72bb4810c6d4119a15b3c33c2c675def6339942a7b30a92135f14915fa0dd67aa327f096131516b8bb29233a93eb0d5bd855f23ab73dd6805f48efc807ba0753d4f821a061eb60ee73aeb2cd809afdb32f556eb4280163088f0be9c756b40a0be3bdee948a810894269d55a5bf1ca2e04885e55bd24593aeda83d6dc93125d6a0bcd8795c0f1a54c18178a50490156ce0e37acd74a873819f0f8dbca019605183a0d99c3250fdb75bee213232c94d9774e5c65eab1c94e6a718e93a88ee0362e0a8a00ca7288fe5a5a8eff7194dbeb64a5f15356c02b5071e8afad01d46c6e66c890fa05843c2b0ee571052aebcb000e8aad739e30765e5cb1fecbb29b49584d3cefa60a0ac2c97fee3a54eb375a334e6a594e5412d82eec5ea7a2caaed699c3850369a0680",
            "0xf90211a0007dd7dbd837aed139221f449a71d0b4d8e8d496b62f515192f89377b7a54ce8a0816a701e02bc1484d167691ec621e91e3414034cec908cc400545a318a464b93a04eda27504c75434d02b26ff6f9cad4e7d9d85ce547bcfa9fb8a378b3c8104f2ca03bb6d6053b6075b7052c729a4ed73886abb90dae9609fcaf53caa1a2bd7fe9dca02a13f5b4a8f3c96922599f11713be6087a9fc7d7b715d2e42fe46c915ea4279fa0bd311e57cc8d62cbade777276d4b9d6a6a687ee6ef5d9c80c8280eb7d398d3c4a0f26e7856776c59abba1af1d80ebfde451f495342bcd4fedead55cc64cbb1fbf5a0d0cd9d93b6b192d87f66c43a758ea0dc9b7f75f26aeaef0466768cfa85be9f60a0b78a04c9a3f93c367e060ad7cf1b8dba2acda2aa525a56a064d1cb37d83c7a4aa0994446f7df83c740cd40444d2c18106d6ccdc36dcc99f4fa30b92be7661046eda08c63918e11055075450df554f632c2f009f8d9392595d5b52424bc2a87516734a045a9142bf2ebb004af147eb1db5ee2a569c24c2752dad628c521df69a484bc33a0c6785918addcd45644348c16b8fc9fa6d429d6f395d5c393e8f38982b1262357a08d552812d68aac20d6ea9641247ce9b0c7811416f7d8d25163eb8b8e2deb9f72a0233f7d0b62f66ebad765da6f63a284e9c1a519dfb17cd50c8d85c87d12dafaf1a0212ae8f250f899a4b871a534548836a64ee93aae083d5e596a84cff533ceaa5a80",
            "0xf8f180a0d7d633a58946d00ec7a1f93c1ca631df0a8673eae96370b6b72631a2907dfb77a020109805f7e2c937b8f329a424fecbb2a67ec3ccd55b48f7ead0dcca74c73d6480a0b40cabe387ab1ae0428e6acd4f45febed642e349e6ff78e5a0c05562312e524fa08ea99d2bb64a345ce2b71b68bca347bce6b8682a48398e889e78167d989067d58080a06da6a64497a321688bf2df91fadc67562ad61132cff63169753fccb53ead595e80a04145b159ba7e409b02abc554aa956278ef224a1b19afc7c6ffb435eb7585a59e8080a0aa7d188521de91c9d7605a4c7529e733f0a67d82f5fbab4c0a1435c8d4bd2afc808080",
            "0xf8669d338cfc997a82252167ac25a16580d9730353eb1b9f0c6bbf0e4c82c4d0b846f8440180a0199c2e6b850bcc9beaea25bf1bacc5741a7aad954d28af9b23f4b53f5404937ba04e36f96ee1667a663dfaac57c4d185a0e369a3a217e0079d49620f34f85d1ac7"
        ],
        "address": "0x6b175474e89094c44da98b954eedeac495271d0f",
        "balance": "0x0",
        "codeHash": "0x4e36f96ee1667a663dfaac57c4d185a0e369a3a217e0079d49620f34f85d1ac7",
        "nonce": "0x1",
        "storageHash": "0x199c2e6b850bcc9beaea25bf1bacc5741a7aad954d28af9b23f4b53f5404937b",
        "storageProof": [
        ]
        });
        let storage_keys =
            vec!["0x2045bf4ea5561e88a4d0d9afbc316354e49fe892ac7e961a5e68f1f4b9561152".to_owned()];

        let res = calculate_proof(&trie_proof, storage_keys);

        assert!(res.is_err());
    }

    #[test]
    fn storage_and_account_empty() {
        let trie_proof = serde_json::json!({
        "accountProof": [
        ],
        "address": "0x6b175474e89094c44da98b954eedeac495271d0f",
        "balance": "0x0",
        "codeHash": "0x4e36f96ee1667a663dfaac57c4d185a0e369a3a217e0079d49620f34f85d1ac7",
        "nonce": "0x1",
        "storageHash": "0x199c2e6b850bcc9beaea25bf1bacc5741a7aad954d28af9b23f4b53f5404937b",
        "storageProof": [
        ]
        });
        let storage_keys =
            vec!["0x2045bf4ea5561e88a4d0d9afbc316354e49fe892ac7e961a5e68f1f4b9561152".to_owned()];

        let res = calculate_proof(&trie_proof, storage_keys);

        assert!(res.is_err());
    }
}