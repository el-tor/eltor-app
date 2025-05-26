El Tor - Paid Circuit Protocol 
==============================


Client Flow
==========
A client wants to build a paid circuit:


<b>1. Relay Descriptor Lookup</b>

Lookup Relays:

- `PAYMENT_RATE` - rate the relay charges in msats per payment interval (default=1000)
- `PAYMENT_INTERVAL` - seconds per each payment interval (default=60)
- `PAYMENT_INTERVAL_MAX_ROUNDS` - how many rounds of payments before the circuit is killed (default=10). max is 10 due to limits on data we can pass in a tor onion cell
- `HANDSHAKE_FEE` - a fee in msats the relay might charge to do bandwidth testing in the pre-build handshake step (default=0)
- `BANDWIDTH_QUOTA` - a quota set in KBytes on how much bandwidth a client can use per payment interval. *future work, not being used yet (default=0) unlimited

Select Relays:

Select 3 (or more) relays (entry,middle,exit) and lookup the BOLT 12 offer (or lnurl*) to pay. 
This info can be found locally in the `cached-microdesc-consensus` file that is downloaded from the Directory Authorities.
Relay selection is a topic for more research. For now, it's a random algo, excluding relays that charge exuberant fees. 
A client can set a `CIRCUIT_MAX_FEE` in msats to stay under. Maybe something like 20 sats for a 10 min circuit. 
It makes reasonable sense for a user to desire to keep monthly expenses to about what they are willing to pay for a centralized VPN service (average $10/month).
Maybe they will pay for privacy and splurge to $20-$30 a month. Since El Tor is based on usage, this amount can vary based on how long you use a circuit. A
future iteration could even include have a `BANDWIDTH_QUOTA` per circuit. 

<b>2. Handshake Fee</b>

During the handshake in the circuit build step in tor, a client can test the bandwidth for the circuit. 
A relay can charge a fee for this step and there are incentives on why, or why not, the relay may want to charge a fee. 

Fee incentives:

A relay charging a handshake fee might make sense in a few senerios:
- if you are a mature relay that a lot of clients already trust you might want to charge a fee. The client will be willing to pay this fee
for the relays high service level and honest bandwidth.
- to prevent "free loader" clients. A "free loader" client is one that builds a circuit (without a handshake fee), uses it for free until payment is 
required for the first round beyond the inital bandwidth test. Then the free loader prematurley kills the circuit before paying the first interval round. 
In Tor, relays cannot see the client IP (unless you are a guard), so there is no simple way to prevent a "free loader" except charging a fee. 
Free loading might be discouraged if the relay sets a small enough interval, lets say 10 seconds. This will prevent the free loader from getting any meaningful
bandwidth usage before he is disconnected. But the interval must be long enough to allow a lightning payment to go thru.


No Fee incentives:

Setting a fee probably does not make sense:
- if you are a noobie relay because nobody trusts your advertisted bandwdith yet. The relay might take the fee and run away
- allow clients to do free bandwidth testing 
- if you want more clients connecting to you, leading to higher profits

<b>3. Circuit build</b>

Now that you know the handshake fee the next step is to build the circuit:

- a. Config 
    - a1. A typical relay might charge 1000 msats `PAYMENT_RATE` per interval of 60 seconds `PAYMENT_INTERVAL` up to 10 minutes `PAYMENT_INTERVAL_MAX_ROUNDS`. 
    Typically the `HANDSHAKE_FEE` is 0.
    - a2. A `HANDSHAKE_FEE` might be required by the relay
- b. Lightning Payment Setup
    - b1. For BOLT 12 Offer: Create 10 random `PAYMENT_ID` hashes (32 byte) for each relay to include later (see step 6) in the encypted onion message of a BOLT 12 payment.
    Make sure each `PAYMENT_ID` is unique to avoid giving up privacy in correlation attempts.
    - b2. For LNURL: create 10 invoices and concatinate the 10 payment hashes as the `PAYMENT_ID`, in the chronological order that you are going to pay them for each interval.
- c. Handshake fee
    - c1. If NO handshake fee is required the `handshake_fee_payment_hash` and `handshake_fee_preimage` is a random hash. This is a dummy hash to pad for privacy reasons to prevent
    against a malicious relay that might contol two hops to prevent correlation and timing attacks.
    - c2. If a handshake fee is required, then the client pays the relay out of band and inserts a valid payment proof in the `handshake_fee_payment_hash` and `handshake_fee_preimage`
- d. Call the <b>EXTENDPAIDCIRCUIT</b> RPC
    ```
    EXTENDPAIDCIRCUIT 0
    fingerprint_entry_guard handshake_fee_payment_hash handshake_fee_preimage 10_payment_ids_concatinated
    fingerprint_middle_relay handshake_fee_payment_hash handshake_fee_preimage 10_payment_ids_concatinated
    fingerprint_exit_relay handshake_fee_payment_hash handshake_fee_preimage 10_payment_ids_concatinated
    ```

<b>4. Test Bandwidth</b>

Test that the circuit's bandwidth is as advertised for the first 10 seconds or so (make sure to pay before 1 min (or interval due) , *remember that it might take some time to find a lightning route).

<b>5. Init Payments Ledger</b>

Add the newly built circuit data to the `payments-ledger` to track each round by `CIRC_ID` and `PAYMENT_ID`. 
Extract each of the 10 `PAYMENT_ID`'s from the RPC call and assisgn a round number to each in order. 
Record 0 for each `UPDATED_AT` to signal that the round has not been paid yet.
See diagram below:

`payments-ledger`
```
PAYMENT_ID    CIRC_ID          ROUND      RELAY_FINGERPRINT    UPDATED_AT  
------------- -----------  ------------   -----------------   ------------  
   111           456             1             ENTRY_N             0
   222           456             1             MIDDLE_N            0
   333           456             1             Exit_N              0
   444           456             2             ENTRY_N             0
   555           456             2             MIDDLE_N            0
   777           456             2             Exit_N              0
   ...           ...             .                .                .
   999           456             10                                0
```

This kicks off the "Client Bandwidth Watcher".

<b>6. Client Bandwidth Watcher </b>

The client watcher is responsible for testing bandwidth every interval and handles payment for the next round of bandwidth.

```
LOOP every interval (1 min default) up until MAX rounds (10 default), then kill circuit
    - if good bandwidth 
        - then pay each relay with their respective PAYMENT_ID for the ROUND 
    - else if bad bandwidth - kill circuit
LOOP
```

<b>6. Repeat</b>

After the circuit is expired, build a new one and repeat. 

Relay Flow
=========
A relay configures his payment preferences and rate then wants to start sharing his bandwidth. 

<b>1. Config</b>

torrc
```
# Static lightning offer code
PAYMENT_BOLT12_OFFER lno***

# BIP-353 name that uses DNS to map to a BOLT 12 offer
PAYMENT_BOLT12_BIP353 name@domain.com

# Rate the relays charges in msats per payment interval (default=1000)
PAYMENT_RATE 1000

# Seconds per each payment interval (default=60)
PAYMENT_INTERVAL 60

# How many rounds of payments before the circuit is killed (default=10). max is 10 due to limits on data we can pass in a tor onion cell.
PAYMENT_INTERVAL_MAX_ROUNDS 10

# We recommend to set this 0 to allow the client to test the bandwidth. 
# Setting this might make your relay less desirable as a noobie relay, but can be useful if you are being spammed or are a mature relay
HANDSHAKE_FEE 0 

# A quota set in KBytes on how much bandwidth a client can use per payment interval. *future work, not being implemented yet (default=0) unlimited
BANDWIDTH_QUOTA 0
```

Eventually support BOLT 11 because some implementations support blinded paths!
```
PAYMENT_BOLT11_LNURL lnurl*** 
PAYMENT_BOLT11_LIGHTNING_ADDRESS name@domain.com
```

<b>2. Handshake</b>

A relay receives a handshake cell to extend/create a circuit from a client willing to pay for bandwidth.

Handshake Fee check: A relay receives an onion message to `EXTENDPAIDCIRCUIT`. Verify the `handshake_fee_payment_hash` and `handshake_fee_preimage`. 
If no fee is required you should still check this, in the next step the relay watcher will simply ignore the fee since its a dummy hash.

<b>3. Emit Event</b>

The tor daemon will emit the event `EXTEND_PAID_CIRCUIT` for the relay watcher to verify against the lighting database.

<b>4. Relay Event Watcher</b> 

The relay has an event watcher running (currently python) that tracks payments and verifies against the remote lightning database. If also listens for events emitted
from the tor daemon `EXTEND_PAID_CIRCUIT`. The relay event wacher can write to a `payments-ledger` 

On `EXTEND_PAID_CIRCUIT` event received:
- If the relay requires a handshake fee then check that the payment `handshake_fee_payment_hash` is valid and belongs to you by checking the lightning database. 
    - If not valid, then kill the circuit. 
    - If good, then add the newly built circuit to the `payments-ledger` to track each round (in step 5)

<b>5. Init Payments Ledger</b>

Add the newly built circuit data to the `payments-ledger` to track each round by `CIRC_ID` and `PAYMENT_ID`. 
Extract each of the 10 (or `PAYMENT_INTERVAL_MAX_ROUNDS`) `PAYMENT_ID` from event and assisgn a round number to each in order. 
Record 0 for each `UPDATED_AT` to note that the round has not been paid yet. Mark the  `RELAY_FINGERPRINT` as `ME` since you are the one getting paid.
See diagram below:

`payments-leger`
```
PAYMENT_ID    CIRC_ID          ROUND      RELAY_FINGERPRINT    UPDATED_AT  
------------- -----------  ------------   -----------------   ------------   
   111           456             1               ME                0
   222           456             2               ME                0
   ...           ...             .               ME                0
   999           456             10              ME                0
```

<b>6. Lightning Watcher</b>

After you add each `PAYMENT_ID` to the `payments-ledger` kick off a "lightning watcher".

Watch your lightning node for incoming payments that includes a 32 byte `PAYMENT_ID` hash inside a BOLT 12 message (payer_note).
Also watch BOLT 11 invoices,if used, for that same `PAYMENT_ID`, but this value is actually the payment_hash of the invoice.

Payment Proof:

When a lightning payment id (hash) matches then call the `Payment Ledger Cron` in step 7.


<b>7. Payment Ledger Cron (Auditor Loop)</b>

A loop is running every minute (or configured interval) to audit the payments ledger to make sure the pay streams are coming in for active circuits.
```
Loop each `CIRC_ID` and find the smallest round that has an `UPDATED_AT` field that is not 0 (if none then its the first ROUND, no payments received yet)
    ROUND=N
    check in the lightning database if the invoice with that `PAYMENT_ID` was paid:
    if yes paid
        - check if you missed the payment window (NOW - UPDATED_AT > 1 min (or PAYMENT_INTERVAL) )
            - if yes, missed payment window, then kill the circuit and remove the rows from the ledger for the CIRC_ID
            - else 
                Update the UPDATED_AT field with the current unix timestamp for the ROUND. thus incrementing the ROUND
                - if round is greater than or equal to 10 (or MAX_ROUNDS) 
                    - if yes, then kill the circuit and remove the rows from the ledger for the CIRC_ID
                    - if no, return
    if no 
        - check if you are out of bounds of the payment window (NOW - UPDATED_AT > 1 min (or PAYMENT_INTERVAL) )
            - if no, return
            - if yes, then kill the circuit and remove the rows from the ledger for the CIRC_ID
LOOP
```











BOLT 12 Tests between implementations
-------------------------------------
Can we send a message with 32 byte PAYMENT_ID in each of the implementations?
```
CLN
    - CREATE: lightning-cli --network regtest offer 0 clndesc
        - lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzrcgqq9qwcmvdejx2umrzcss87xuqlyz59vpzvw72zc5pzh5jx2mmz6mj7lhxxefacqjdeh6p2hg
    - LIST: lightning-cli --network regtest listinvoices
    - PAY: lightning-cli fetchinvoice -k "offer"="lno***" "amount_msat"="2000" "payer_note"="<PAYMENT_ID>"

Phoenixd 
    - CREATE: Autogenerated or with elcair `./bin/eclair-cli eclair1 tipjarshowoffer`
        - lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzrc2zfjx7mnpw35k7m3qw3hjqetrd3skjuskyyp78pgmys9ygquhdp9xypvfclt02vnf3nynz3ww3pst6l76h565vnc
    - LIST: Subscribe to webhook: `websocat --basic-auth :<phoenixd_api_password> ws://127.0.0.1:9740/websocket`
    - PAY: curl -X POST http://localhost:9740/payoffer \
            -u :<phoenixd_api_password> \
            -d amountSat=2 \
            -d offer=lno1qgsyxjtl6luzd9t3pr62xr...9ry9zqagt0ktn4wwvqg52v9ss9ls22sqyqqestzp2l6decpn87pq96udsvx
            -d message='<PAYMENT_ID>'
LNDK
    - CREATE: ./bin/ldknode-cli lndk1 offer
    - LIST: ??
    - PAY: ??

Strike (only works with LNURL currently)
    TODO TEST LNURL + Blinded Paths

```

- CLN -> Phoenixd
    - [X] BOLT12 message works
    - [X] Message Field Name=payer_note
 
- CLN -> LNDK
    - [ ] BOLT12 message works
    - [ ] Message Field Name=?

- Phoenixd -> CLN
    - [X] BOLT12 message works
    - [X] Message Field Name=payerNote

- Phoenixd -> LNDK
    - [ ] BOLT12 message works
    - [ ] Message Field Name=?

- LNDK -> CLN
    - [ ] BOLT12 message works
    - [ ] Message Field Name=?

- LNDK -> Phoenixd
    - [ ] BOLT12 message works
    - [ ] Message Field Name=?

- Strike -> CLN
    - [ ] BOLT12 message works
    - [ ] Message Field Name=?

- Strike -> Phoenixd
    - [ ] BOLT12 message works
    - [ ] Message Field Name=?

- Strike -> LNDK
    - [ ] BOLT12 message works
    - [ ] Message Field Name=?


### Compatiabilty issue notes

- CLN (current Umbrel version) cannot pay a lno without a description. Phoenixd does not include an offer description when they automatically create the BOLT 12 offer.






Experiments
=============
```
BOLT12
------
Since BOLT12 invoices are static and some implementations do not use fetchinvoice to get a fresh lni with PAYMENT_HASHES, we can generate 10 PAYMENT_IDS locally for the relay to lookup out of band via the offers message.
PAYMENT_IDS=32 bytes (64 chars)
0c38df961d9721a2faf39324c44e575c1dbf7491250d0507316028b8f4315ffd

BOLT11
------
Fetch 10 invoices from the lnurl and concat the 10 payment hashes. The relay will query the lightning database for the payment hashes
PAYMENT_IDS=320 bytes (640 chars)
0c38df961d9721a2faf39324c44e575c1dbf7491250d0507316028b8f4315ff0
11cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e1
21cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e2
31cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e3
41cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e4
51cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e5
61cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e6
71cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e7
81cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e8
91cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e9

0c38df961d9721a2faf39324c44e575c1dbf7491250d0507316028b8f4315ff011cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e121cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e231cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e341cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e451cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e561cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e671cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e781cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e891cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e9
```

```
EXTENDPAIDCIRCUIT 0
fingerprint handshake_fee_payment_hash handshake_fee_preimage 10_payment_ids
fingerprint handshake_fee_payment_hash handshake_fee_preimage 10_payment_ids
fingerprint handshake_fee_payment_hash handshake_fee_preimage 10_payment_ids

EXTENDPAIDCIRCUIT 0
A7649D89B48EEE5FB1C4583C73F8DA6888A19C12 16ea179e9332918b90124b60ecd9b1fe3e08b9e997a058f188ed20cea34a5e0e 68b4e782fafbd5a057ec4c277f01da48db73dd67326ec4458ff89daffba186e3 0c38df961d9721a2faf39324c44e575c1dbf7491250d0507316028b8f4315ff011cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e121cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e231cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e341cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e451cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e561cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e671cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e781cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e891cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e9
52A4FEA9DF61CEBA58C8BF5F1F651A732EFEAB14 16ea179e9332918b90124b60ecd9b1fe3e08b9e997a058f188ed20cea34a5e0e 68b4e782fafbd5a057ec4c277f01da48db73dd67326ec4458ff89daffba186e3 0c38df961d9721a2faf39324c44e575c1dbf7491250d0507316028b8f4315ff011cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e121cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e231cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e341cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e451cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e561cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e671cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e781cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e891cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e9
96DC9F9FAB13614AF6D4451B87BEB9546A9EB8A3 16ea179e9332918b90124b60ecd9b1fe3e08b9e997a058f188ed20cea34a5e0e 68b4e782fafbd5a057ec4c277f01da48db73dd67326ec4458ff89daffba186e3 0c38df961d9721a2faf39324c44e575c1dbf7491250d0507316028b8f4315ff011cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e121cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e231cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e341cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e451cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e561cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e671cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e781cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e891cc244c4e4e2b1d270f60f8cc47e86fedf3503323ad577999b7ab2e993a57e9

```

Alternative ideas
- Send the preimage and payment hash in the data stream. But not every cell, just one per minute. 
- And the relay can verify payment async out of band using the the relay watcher
- Look into creating a custom data command 
- this might be a bad idea because of correlation attacks by adding exta bits to only some data cell packets, but maybe if they are always included and padded it would work - even when payment is not required. just pad with dummy data.
- TYPE can be up to 32 bytes or 64 chars
- DATA can be up to 256 bytes or 512 chars
