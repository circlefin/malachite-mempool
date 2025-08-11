# Malachite consensus engine mempool

A mempool implementation for Malachite BFT Consensus Engine

**Note: This project is currently under active development and is not yet production-ready.**

## Transaction flow

The following diagram shows how a transaction flows through the mempool system:

```mermaid
sequenceDiagram
    participant App as Enpoint
    participant Mempool as Mempool Actor
    participant AppActor as App Actor  
    participant Network as Network Actor
    
    Note over AppActor,Mempool: Initial Setup
    AppActor->>+Mempool: Subscribe(output_port_subscriber)
    Mempool-->>-AppActor: Subscription confirmed
    
    Note over App,Network: Transaction Processing
    App->>+Mempool: Add { tx, reply }
    
    Note over Mempool: Check if mempool is full<br/>(max_pool_size)
    
    alt Mempool Full
        Mempool->>App: Error(MempoolFull)
    else Space Available
        Mempool->>+AppActor: CheckTx { tx, reply } (via output_port)
        
        AppActor->>+AppActor: check_tx(tx)
        Note over AppActor: Deserialize & validate transaction<br/>Always returns outcome if validation succeeds
        
        AppActor->>+Mempool: CheckTxResult { tx, result, reply }
        
        alt Validation Failed
            Mempool->>App: Error(InvalidTransaction)
        else Validation Successul
            Note over Mempool: Outcome received<br/>Check if transaction is valid
            
            alt Transaction Invalid (outcome.is_valid() = false)
                Note over Mempool: Transaction not added to mempool<br/>but still reply with success
                Mempool->>App: Success(tx invalid)
            else Transaction Valid (outcome.is_valid())
                Note over Mempool: Check for duplicates<br/>using tx_hash
            
                alt Duplicate Transaction
                    Note over Mempool: Transaction already exists<br/>in mempool
                    Mempool->>App: Error(TxAlreadyExists)
                else Not Duplicate
                    Mempool->>Mempool: Add to to mempool
                    
                    alt Local Transaction
                        Mempool->>+Network: Broadcast(tx) (gossip)
                        Network-->>-Mempool: Gossip to peers
                    end
                    
                    Mempool->>App: Success(tx valid)
                end
            end
        end
    end
```

**Key behaviors:**
- **Network transactions**: Transactions received from peer nodes follow the same validation flow but are not re-gossiped to prevent network loops and duplicate transaction.
- **Individual gossip**: Each transaction is currently gossiped individually without batching optimization
- **Error types**: The mempool returns specific `MempoolError` variants:
  - `MempoolFull`: When the mempool has reached its maximum capacity
  - `InvalidTransaction`: When transaction validation fails 
  - `TxAlreadyExists`: When attempting to add a duplicate transaction

## License

Copyright © 2025 Informal Systems AG. All rights reserved.

This repository and its contents, including but not limited to source code, documentation, specifications, and associated materials (collectively, the "Software"), are proprietary to Informal Systems AG (the "Company").

The Software is provided for reference purposes only. No license, express or implied, is granted. You may not copy, modify, distribute, sublicense, or use the Software, in whole or in part, for any purpose—commercial or non-commercial—without the prior written consent of the Company.

Unauthorized use of this Software is strictly prohibited.
