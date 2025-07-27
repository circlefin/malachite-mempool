use crate::utils::create_nodes;
use host_integration::node::TestTx;
use std::time::Duration;
use tokio::time::sleep;

pub mod utils;

#[tokio::test]
async fn test_three_node_gossip_and_removal() {
    println!("Starting three-node tx gossip and removal test with actors...");

    let mut nodes = create_nodes(3, 8000).await;

    // Wait a bit for actors to initialize and connect
    sleep(Duration::from_millis(300)).await;

    // Test `add_tx() and `Msg::Add`
    // Each node adds a unique transaction (should trigger gossip)
    println!("Adding transactions and trigger gossip...");
    let tx1 = TestTx(1001);
    let tx2 = TestTx(1002);
    let tx3 = TestTx(1003);

    nodes[0].add_tx(tx1.clone()).await;
    nodes[1].add_tx(tx2.clone()).await;
    nodes[2].add_tx(tx3.clone()).await;

    // Wait for transactions to be processed and gossiped
    sleep(Duration::from_millis(200)).await;

    // Test `get_transactions()` and `Msg::Take`
    // Check state - each node should have all transactions
    for (i, node) in nodes.iter().enumerate() {
        let txs = node.get_transactions().await;
        println!("Node {} has {} transactions", i, txs.len());
        assert_eq!(
            txs.len(),
            3,
            "Node {i} should have exactly 3 transactions initially"
        );
    }

    // Test `remove_tx()` and `Msg::Remove`
    // Remove tx1 from all nodes
    println!("Testing removal...");
    let tx_to_remove = tx1.clone();
    for node in &mut nodes {
        node.remove_tx(&tx_to_remove).await;
    }

    // Wait for removal to be processed
    sleep(Duration::from_millis(200)).await;

    // Check final state after removal
    for (i, node) in nodes.iter().enumerate() {
        let txs = node.get_transactions().await;
        println!("Node {} has {} transactions after removal", i, txs.len());
        assert_eq!(
            txs.len(),
            2,
            "Node {i} should have exactly 2 transactions initially"
        );
    }
}
