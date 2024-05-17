use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod};
use bitcoin::{consensus::Decodable, Transaction, Address, params::Params};

#[derive(Debug, Clone)]
struct BitcoinTxError;

#[ic_cdk::update]
/// The function returns the Bitcoin addresses of the inputs in the
/// transaction with the given transaction ID.
async fn get_inputs(tx_id: String) -> Vec<String> {
    match get_inputs_internal(tx_id).await {
        Ok(inputs) => inputs,
        Err(_) => vec![]
    }
}

async fn get_inputs_internal(tx_id: String) -> Result<Vec<String>, BitcoinTxError> {
    let tx = get_tx(&tx_id).await?;

    let mut addresses = vec![];

    for input in tx.input.iter() {
        let input_tx_id = input.previous_output.txid;
        let input_tx = get_tx(&input_tx_id.to_string()).await?;
        let output = &input_tx.output[input.previous_output.vout as usize];
        let address = Address::from_script(&output.script_pubkey, Params::MAINNET).map_err(|_| BitcoinTxError)?;
        addresses.push(address.to_string());
    }

    Ok(addresses)
}

async fn get_tx(tx_id: &String) -> Result<Transaction, BitcoinTxError> {
    let host = "blockstream.info";
    let url = format!("https://{}/api/tx/{}/raw", host, tx_id);
    let request_headers = vec![
        HttpHeader {
            name: "Host".to_string(),
            value: format!("{host}:443"),
        },
        HttpHeader {
            name: "User-Agent".to_string(),
            value: "bitcoin_inputs_collector".to_string(),
        },
    ];
    let request = CanisterHttpRequestArgument {
        url: url.to_string(),
        method: HttpMethod::GET,
        body: None,
        max_response_bytes: Some(400 * 1024), // 400 KiB
        transform: None,
        headers: request_headers,
    };
     let cycles = 49_140_000 + 1024 * 5_200 + 10_400 * 400 * 1024;  // 1 KiB request, 400 KiB response
     match http_request(request, cycles).await {
        Ok((response,)) => {
            let tx = Transaction::consensus_decode(&mut response.body.as_slice()).map_err(|_| BitcoinTxError)?;
            // Verify the correctness of the transaction by recomputing the transaction ID.
            if tx.compute_txid().to_string() != *tx_id {
                return Err(BitcoinTxError);
            }
            Ok(tx)
        }
        Err((r, m)) => {
            println!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}");
            Err(BitcoinTxError)
        }
    }
}
