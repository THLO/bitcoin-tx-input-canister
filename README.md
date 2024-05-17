# bitcoin-tx-input-canister

This is a simple, proof-of-concept canister that returns the input addresses of
a given Bitcoin transaction, making use of [HTTPS outcalls](https://internetcomputer.org/https-outcalls).

It exposes the following endpoint:

```bash
type tx_id = text;
type bitcoin_address = text;
"get_inputs": (tx_id) -> (vec bitcoin_address);
```

> [!CAUTION]  
> This is merely a proof of concept. The code is not ready for use in production.
> In particular, the canister depends on the availability of the [Blockstream API](https://github.com/Blockstream/esplora/blob/master/API.md), i.e., there is a single point of failure.
