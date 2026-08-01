// SPDX-License-Identifier: MPL-2.0
//! Shared λδ vectors consumed by native Rust and the browser WASM boundary.

use nexia_core::lambdadelta::{Budget, Interp};
use serde::Deserialize;

#[derive(Deserialize)]
struct Vector {
    name: String,
    source: String,
    printed: String,
}

#[test]
fn native_kernel_matches_shared_vectors() {
    let vectors: Vec<Vector> = serde_json::from_str(include_str!(
        "../../tests/fixtures/lambdadelta-conformance.json"
    ))
    .expect("conformance fixture must be valid JSON");

    for vector in vectors {
        let result = Interp::new()
            .eval_str(&vector.source, Budget::new())
            .unwrap_or_else(|error| panic!("{} failed: {error}", vector.name));
        assert_eq!(result.to_string(), vector.printed, "{}", vector.name);
    }
}
