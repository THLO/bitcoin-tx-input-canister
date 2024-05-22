use bitcoin::{consensus::Decodable, params::Params, Address, Transaction};
use futures::future::try_join_all;
use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,
    TransformContext, TransformFunc,
};

#[derive(Debug, Clone)]
struct BitcoinTxError;

#[ic_cdk::update]
/// The function returns the Bitcoin addresses of the inputs in the
/// transaction with the given transaction ID.
async fn get_inputs(tx_id: String) -> Vec<String> {
    match get_inputs_internal(tx_id).await {
        Ok(inputs) => inputs,
        Err(_) => vec![],
    }
}

async fn get_inputs_internal(tx_id: String) -> Result<Vec<String>, BitcoinTxError> {
    let tx = get_tx(tx_id).await?;

    let mut addresses = vec![];
    let mut futures = vec![];
    let mut vouts = vec![];

    for input in tx.input.iter() {
        vouts.push(input.previous_output.vout as usize);
        futures.push(get_tx(input.previous_output.txid.to_string()));
    }
    let input_txs = try_join_all(futures).await?;

    for (index, input_tx) in input_txs.iter().enumerate() {
        let output = &input_tx.output[vouts[index]];
        let address = Address::from_script(&output.script_pubkey, Params::MAINNET)
            .map_err(|_| BitcoinTxError)?;
        addresses.push(address.to_string());
    }

    Ok(addresses)
}

async fn get_tx(tx_id: String) -> Result<Transaction, BitcoinTxError> {
    let host = "btcscan.org";
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
        transform: Some(TransformContext {
            function: TransformFunc(candid::Func {
                principal: ic_cdk::api::id(),
                method: "transform".to_string(),
            }),
            context: vec![],
        }),
        headers: request_headers,
    };
    let cycles = 49_140_000 + 1024 * 5_200 + 10_400 * 400 * 1024; // 1 KiB request, 400 KiB response
    match http_request(request, cycles).await {
        Ok((response,)) => {
            let tx = Transaction::consensus_decode(&mut response.body.as_slice())
                .map_err(|_| BitcoinTxError)?;
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

#[ic_cdk::query]
fn transform(raw: TransformArgs) -> HttpResponse {
    HttpResponse {
        status: raw.response.status.clone(),
        body: raw.response.body.clone(),
        headers: vec![],
    }
}
