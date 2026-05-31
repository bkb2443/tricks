/// TypeScript binding generation test.
///
/// Run `cargo test export_typescript_bindings` to regenerate `client/src/engine/types.ts`.
///
/// NOTE: This test requires a `src/lib.rs` that re-exports the public API (see lib.rs).
#[test]
fn export_typescript_bindings() {
    use std::fmt::Write as FmtWrite;
    use ts_rs::TS;

    // Import types from the library crate
    use tricks_server::engine::bid::BidPayload;
    use tricks_server::engine::meta::{EuchreMeta, LobbyMeta, SheepsheadMeta};
    use tricks_server::engine::state::{ChatMessage, ClientMessage, HintCard, SeatInfo};
    use tricks_server::engine::{
        Card, GameMeta, GamePhase, GameState, Rank, StateUpdate, Suit, Trick,
    };

    let mut output = String::new();
    writeln!(
        output,
        "// AUTO-GENERATED — do not edit by hand.\n\
         // Run `cargo test export_typescript_bindings` in server/ to regenerate.\n"
    )
    .unwrap();

    // JsonValue is referenced by the `serde_json::Value` fields (Bid.value, BidPlaced.value).
    // ts-rs maps serde_json::Value → JsonValue but does not emit the definition itself,
    // so we prepend it here manually.
    writeln!(output, "type JsonValue = number | string | boolean | null | JsonValue[] | {{ [key: string]: JsonValue }};").unwrap();
    writeln!(output).unwrap();

    // Helper to turn a ts-rs `decl()` string (e.g. `type Foo = ...;`) into an exported one.
    fn to_export(decl: String) -> String {
        if decl.starts_with("type ") {
            format!("export {decl}\n")
        } else {
            decl
        }
    }

    output.push_str(&to_export(Suit::decl()));
    output.push_str(&to_export(Rank::decl()));
    output.push_str(&to_export(Card::decl()));
    output.push_str(&to_export(Trick::decl()));
    output.push_str(&to_export(GamePhase::decl()));
    output.push_str(&to_export(SeatInfo::decl()));
    output.push_str(&to_export(ChatMessage::decl()));
    output.push_str(&to_export(LobbyMeta::decl()));
    output.push_str(&to_export(SheepsheadMeta::decl()));
    output.push_str(&to_export(EuchreMeta::decl()));
    output.push_str(&to_export(GameMeta::decl()));
    output.push_str(&to_export(HintCard::decl()));
    output.push_str(&to_export(GameState::decl()));
    output.push_str(&to_export(BidPayload::decl()));
    output.push_str(&to_export(ClientMessage::decl()));
    output.push_str(&to_export(StateUpdate::decl()));

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("server/ must have a parent directory")
        .join("client/src/engine/types.ts");

    std::fs::write(&path, &output)
        .unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));

    println!("TypeScript bindings written to {}", path.display());
}
