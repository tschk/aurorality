use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::{json, Value};
// We can't easily include hyperchat_backend without editing Cargo.toml, so let's just bench the logic directly.

#[derive(Debug, Clone)]
struct TransportMessage {
    id: String,
    text: String,
    transport: String,
    status: String,
    metadata: Value,
}

fn old_sync(sync: &Value, room_id: &str) -> Vec<TransportMessage> {
    let mut messages = Vec::new();
    if let Some(timeline) = sync["rooms"]["join"][room_id]["timeline"]["events"].as_array() {
        for event in timeline {
            if event["type"].as_str() == Some("m.room.message") {
                let body = event["content"]["body"]
                    .as_str()
                    .unwrap_or("(empty)")
                    .to_string();
                messages.push(TransportMessage {
                    id: event["event_id"].as_str().unwrap_or("?").to_string(),
                    text: body,
                    transport: "matrix".to_string(),
                    status: "synced".to_string(),
                    metadata: event.clone(),
                });
            }
        }
    }
    messages
}

fn new_sync(sync: &mut Value, room_id: &str) -> Vec<TransportMessage> {
    let mut messages = Vec::new();
    if let Value::Array(timeline) = sync["rooms"]["join"][room_id]["timeline"]["events"].take() {
        for event in timeline {
            if event["type"].as_str() == Some("m.room.message") {
                let body = event["content"]["body"]
                    .as_str()
                    .unwrap_or("(empty)")
                    .to_string();
                let event_id = event["event_id"].as_str().unwrap_or("?").to_string();
                messages.push(TransportMessage {
                    id: event_id,
                    text: body,
                    transport: "matrix".to_string(),
                    status: "synced".to_string(),
                    metadata: event,
                });
            }
        }
    }
    messages
}

fn bench_matrix_sync(c: &mut Criterion) {
    let events = (0..100)
        .map(|i| {
            json!({
                "type": "m.room.message",
                "content": {"body": format!("Message {}", i)},
                "event_id": format!("$event_{}", i)
            })
        })
        .collect::<Vec<_>>();

    let sync_data = json!({
        "rooms": {
            "join": {
                "room_1": {
                    "timeline": {
                        "events": events
                    }
                }
            }
        }
    });

    c.bench_function("matrix_sync_old", |b| {
        b.iter(|| old_sync(black_box(&sync_data), "room_1"));
    });

    c.bench_function("matrix_sync_new", |b| {
        b.iter_batched(
            || sync_data.clone(),
            |mut data| new_sync(black_box(&mut data), "room_1"),
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_matrix_sync);
criterion_main!(benches);
