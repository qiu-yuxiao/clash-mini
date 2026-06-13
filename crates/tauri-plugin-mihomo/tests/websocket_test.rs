use std::{fmt::Debug, sync::Arc, time::Duration};

use tauri_plugin_mihomo::{
    Result,
    models::{Connections, Log, LogLevel, Memory, Traffic},
};

mod common;

fn handle_message<T: Debug + serde::de::DeserializeOwned>(data: Vec<u8>) {
    match String::from_utf8(data) {
        Ok(text) if text.starts_with("Websocket error") || text.starts_with("websocket error") => {
            println!("received error: {text}");
        }
        Ok(text) => match serde_json::from_str::<T>(&text) {
            Ok(data) => println!("{data:?}"),
            Err(error) => println!("failed to parse websocket payload: {error}; payload={text}"),
        },
        Err(error) => println!("failed to decode websocket payload: {error}"),
    }
}

#[tokio::test]
async fn mihomo_websocket_memory() -> Result<()> {
    let mihomo = common::mihomo();
    let websocket_id = mihomo
        .ws_memory(move |data| {
            handle_message::<Memory>(data);
        })
        .await?;
    println!("WebSocket ID: {websocket_id}");
    tokio::time::sleep(Duration::from_millis(5000)).await;
    mihomo.disconnect(websocket_id, Some(0)).await?;
    for i in 0..10 {
        println!("check connection exist {i}");
        let manager = Arc::clone(&mihomo.connection_manager);
        let manager = manager.0.read().await;
        if manager.get(&websocket_id).is_none() {
            println!("connection exist");
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    Ok(())
}

#[tokio::test]
async fn mihomo_websocket_traffic() -> Result<()> {
    let mihomo = common::mihomo();
    let websocket_id = mihomo
        .ws_traffic(move |data| {
            handle_message::<Traffic>(data);
        })
        .await?;
    println!("WebSocket ID: {websocket_id}");
    tokio::time::sleep(Duration::from_millis(5000)).await;
    mihomo.disconnect(websocket_id, Some(0)).await?;
    for i in 0..10 {
        println!("check connection exist {i}");
        let manager = Arc::clone(&mihomo.connection_manager);
        let manager = manager.0.read().await;
        if manager.get(&websocket_id).is_none() {
            println!("connection exist");
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    Ok(())
}

#[tokio::test]
async fn mihomo_websocket_log() -> Result<()> {
    let mihomo = common::mihomo();
    let websocket_id = mihomo
        .ws_logs(LogLevel::DEBUG, move |data| {
            handle_message::<Log>(data);
        })
        .await?;
    println!("WebSocket ID: {websocket_id}");
    tokio::time::sleep(Duration::from_millis(5000)).await;
    mihomo.disconnect(websocket_id, Some(0)).await?;
    for i in 0..10 {
        println!("check connection exist {i}");
        let manager = Arc::clone(&mihomo.connection_manager);
        let manager = manager.0.read().await;
        if manager.get(&websocket_id).is_none() {
            println!("connection exist");
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    Ok(())
}

#[tokio::test]
async fn mihomo_websocket_connections() -> Result<()> {
    let mihomo = common::mihomo();
    let websocket_id = mihomo
        .ws_connections(move |data| {
            handle_message::<Connections>(data);
        })
        .await?;
    println!("WebSocket ID: {websocket_id}");
    tokio::time::sleep(Duration::from_millis(5000)).await;
    mihomo.disconnect(websocket_id, Some(0)).await?;
    for i in 0..10 {
        println!("check connection exist {i}");
        let manager = Arc::clone(&mihomo.connection_manager);
        let manager = manager.0.read().await;
        if manager.get(&websocket_id).is_none() {
            println!("connection exist");
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    Ok(())
}
