use super::*;

#[test]
fn utility_calculation_bounds() {
    let params = UtilityHyperparams::default();
    let metrics = CandidateMetrics {
        primary_score: 85.0,
        parent_primary_score: Some(70.0),
        children_count: 0,
    };

    let out = UtilityCalculator::calculate(&metrics, 50.0, 100.0, 0.0, 20.0, &params);
    assert!((out.raw_delta - 15.0).abs() < 1e-6);
    assert!((out.novelty_score - 1.0).abs() < 1e-6);
    assert!((out.s_norm - 0.70).abs() < 1e-4);
    assert!(out.composite > 0.0 && out.composite <= 1.0);
}

#[test]
fn cas_blake3_and_zstd_roundtrip() {
    let src = "def solve(arr):\n    return max(arr) if arr else 0\n";
    let h1 = cas::hash_code(src);
    let h2 = cas::hash_code(src);
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64);

    let compressed = cas::compress_code(src).unwrap();
    let restored = cas::decompress_code(&compressed).unwrap();
    assert_eq!(restored, src);
}

#[test]
fn store_lifecycle_frontier_and_parent_sampling() {
    let reg = Registry::open_in_memory().unwrap();
    let params = UtilityHyperparams::default();

    reg.upsert_task(
        "opt-synth",
        "Vector Max",
        "Write solve(arr) returning the maximum element.",
        "import candidate; print('RSI_METRIC_JSON={\"status\":\"SUCCESS\",\"score\":5.0}')",
    )
    .unwrap();

    let c1 = reg
        .record_experience_card(
            &NewCardInput {
                card_id: "card-1",
                task_id: "opt-synth",
                parent_a_id: None,
                parent_b_id: None,
                operator_used: "DRAFT",
                code: "def solve(arr):\n    return sorted(arr)[-1]\n",
                execution_status: "SUCCESS",
                raw_output: "RSI_METRIC_JSON={\"status\":\"SUCCESS\",\"score\":70.0}",
                primary_score: 70.0,
                execution_time_ms: 42,
            },
            &params,
        )
        .unwrap();
    assert_eq!(c1.children_count, 0);

    let c2 = reg
        .record_experience_card(
            &NewCardInput {
                card_id: "card-2",
                task_id: "opt-synth",
                parent_a_id: Some("card-1"),
                parent_b_id: None,
                operator_used: "IMPROVE",
                code: "def solve(arr):\n    return max(arr)\n",
                execution_status: "SUCCESS",
                raw_output: "RSI_METRIC_JSON={\"status\":\"SUCCESS\",\"score\":95.0}",
                primary_score: 95.0,
                execution_time_ms: 12,
            },
            &params,
        )
        .unwrap();
    assert!((c2.delta_score - 25.0).abs() < 1e-6);

    let c1_reloaded = reg.get_card("card-1").unwrap().unwrap();
    assert_eq!(c1_reloaded.children_count, 1);
    assert!((c1_reloaded.novelty_score - 0.5).abs() < 1e-6);

    let frontier = reg.get_frontier("opt-synth").unwrap();
    assert_eq!(frontier.len(), 2);
    assert_eq!(frontier[0].rank, 1);
    assert_eq!(frontier[0].card.card_id, "card-2");

    let sampled = reg
        .sample_parent_by_utility("opt-synth", false, Some("card-2"))
        .unwrap()
        .unwrap();
    assert_eq!(sampled.card_id, "card-1");
}
