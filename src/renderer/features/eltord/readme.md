eltord
======

Note - This will eventually be ported over to a rust app.

Client Flow
------------
A user wants to build a paid circuit:

1. lookup 3 relays and get the bolt12 offers to pay from the `cached-microdesc-consensus` file
2. create a payment id hash for each relay and include it in the message of the bolt12 payment.
3. Call RPC 
```
EXTENDPAIDCIRCUIT 0
fingerprint paymentidhash 
fingerprint paymentidhash 
fingerprint paymentidhash
```
4. Test that the circuit's bandwidth is as advertised for the first 10 seconds.

    <b>Loop</b>

    a. if good - then pay each relay with their respective payment id hash in the memo

    b. if bad - kill circuit

    <b>Loop every minute up until 10, then kill circuit</b>


Relay Flow
------------
A relay submits his bolt12 offer and rate (millisats per minute)
```
SETCONF PaymentInvoice="lno***" // bolt12 offer
SETCONF PaymentRate="1000" // in millisats per minute
```

A relay receives a cell to extend/create a circuit.

1. Create a ledger to track payments like this:
```
paymentidhash lastUsedTimeStamp
------------- -----------------
hash1           null
```
2. On receiving a cell add to this ledger
```
paymentidhash lastUsedTimeStamp
------------- -----------------
hash1           timestamp1
```

3. eltord is always looping and auditing payments in the ledger
```
Loop each paymentIdHash and if lastUsedTimeStamp !== null and lastUsedTimeStamp > 1 min
    check the lightning database that the invoice with that paymentIdHash was paid
    if yes - then update the lastUsedTimeStamp
    if no - then kill the circuit and remove the row from the ledger
LOOP
```