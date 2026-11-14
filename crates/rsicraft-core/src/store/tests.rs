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
                score_std: 0.0,
                runs_count: 1,
                model_generation: 0,
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
                score_std: 0.0,
                runs_count: 1,
                model_generation: 0,
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

fn seed_task(reg: &Registry, task_id: &str) {
    reg.upsert_task(task_id, "T", "Write solve(arr).", "import candidate")
        .unwrap();
}

fn add_card(
    reg: &Registry,
    card_id: &str,
    task_id: &str,
    parent: Option<&str>,
    status: &str,
    score: f64,
) -> StoreResult<ExperienceCard> {
    reg.record_experience_card(
        &NewCardInput {
            card_id,
            task_id,
            parent_a_id: parent,
            parent_b_id: None,
            operator_used: if parent.is_some() { "IMPROVE" } else { "DRAFT" },
            code: &format!("def solve(arr):\n    return {card_id:?}\n"),
            execution_status: status,
            raw_output: "",
            primary_score: score,
            score_std: 0.0,
            runs_count: 1,
            model_generation: 0,
            execution_time_ms: 1,
        },
        &UtilityHyperparams::default(),
    )
}

#[test]
fn failed_cards_do_not_flatten_selection_pressure() {
    let reg = Registry::open_in_memory().unwrap();
    seed_task(&reg, "t");
    add_card(&reg, "slow", "t", None, "SUCCESS", 96.52).unwrap();
    add_card(&reg, "fast", "t", None, "SUCCESS", 96.60).unwrap();
    add_card(&reg, "broken", "t", None, "RUNTIME_ERROR", 0.0).unwrap();

    let bounds = reg.get_task_score_bounds("t").unwrap();
    assert!((bounds.min_score - 96.52).abs() < 1e-9);
    assert_eq!((bounds.total_cards, bounds.success_cards), (3, 2));

    let slow = reg.get_card("slow").unwrap().unwrap();
    let fast = reg.get_card("fast").unwrap().unwrap();
    let broken = reg.get_card("broken").unwrap().unwrap();
    // Normalized over working cards only, the better card is clearly preferred.
    assert!(fast.composite_utility - slow.composite_utility > 0.4);
    // A failed card earns no score term at all, so it never outranks a working one.
    assert!(broken.composite_utility <= slow.composite_utility + 1e-9);
    assert!(broken.composite_utility < fast.composite_utility);
}

#[test]
fn record_rejects_cross_task_parents_and_rolls_back() {
    let reg = Registry::open_in_memory().unwrap();
    seed_task(&reg, "a");
    seed_task(&reg, "b");
    add_card(&reg, "a1", "a", None, "SUCCESS", 1.0).unwrap();

    assert!(add_card(&reg, "b1", "b", Some("a1"), "SUCCESS", 2.0).is_err());
    assert!(add_card(&reg, "b2", "b", Some("missing"), "SUCCESS", 2.0).is_err());
    // Nothing was written and the would-be parent was not touched.
    assert!(reg.get_card("b1").unwrap().is_none());
    assert_eq!(reg.get_card("a1").unwrap().unwrap().children_count, 0);
    assert_eq!(reg.status().unwrap().cards_count, 1);
}

#[test]
fn sampling_never_returns_the_excluded_card() {
    let reg = Registry::open_in_memory().unwrap();
    seed_task(&reg, "t");
    add_card(&reg, "only", "t", None, "SUCCESS", 1.0).unwrap();
    assert!(reg
        .sample_parent_by_utility("t", false, Some("only"))
        .unwrap()
        .is_none());

    add_card(&reg, "bad", "t", None, "COMPILE_ERROR", 0.0).unwrap();
    let faulty = reg
        .sample_parent_by_utility("t", true, None)
        .unwrap()
        .unwrap();
    assert_eq!(faulty.card_id, "bad");
    let working = reg
        .sample_parent_by_utility("t", false, None)
        .unwrap()
        .unwrap();
    assert_eq!(working.card_id, "only");
}

#[test]
fn import_is_atomic_scoped_to_its_task_and_keeps_foreign_keys_on() {
    let reg = Registry::open_in_memory().unwrap();
    let hp = UtilityHyperparams::default();
    seed_task(&reg, "src");
    seed_task(&reg, "other");
    add_card(&reg, "s1", "src", None, "SUCCESS", 5.0).unwrap();
    add_card(&reg, "s2", "src", Some("s1"), "SUCCESS", 7.0).unwrap();
    add_card(&reg, "o1", "other", None, "SUCCESS", 3.0).unwrap();
    let mut bundle = reg.export_task_bundle("src").unwrap();
    assert!(reg.delete_task("src").unwrap());

    // A bundle with one invalid card must not import anything.
    let mut bad = bundle.clone();
    bad["cards"][0]["execution_status"] = "BOGUS".into();
    assert!(reg.import_task_bundle(&bad, &hp).is_err());
    assert!(reg.get_task("src").unwrap().is_none());

    // Cards claiming another task, or pointing at a parent that does not
    // exist, are filed under the bundle's task with the dangling edge cleared.
    for card in bundle["cards"].as_array_mut().unwrap() {
        card["task_id"] = "other".into();
        if card["card_id"] == "s1" {
            card["parent_a_id"] = "ghost".into();
        }
    }
    assert_eq!(reg.import_task_bundle(&bundle, &hp).unwrap(), 2);
    assert_eq!(reg.list_cards("src", None, None, 10).unwrap().len(), 2);
    assert_eq!(reg.list_cards("other", None, None, 10).unwrap().len(), 1);
    let s1 = reg.get_card("s1").unwrap().unwrap();
    assert_eq!(s1.parent_a_id, None);
    let s2 = reg.get_card("s2").unwrap().unwrap();
    assert_eq!(s2.parent_a_id.as_deref(), Some("s1"));
    assert_eq!(reg.get_frontier("src").unwrap().len(), 2);

    // Foreign keys are still enforced on this connection afterwards.
    let fk_on: i64 = reg
        .conn
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(fk_on, 1);
    assert!(reg
        .conn
        .execute("DELETE FROM tasks WHERE task_id = 'src'", [])
        .is_err());
}

#[test]
fn prune_and_delete_leave_a_consistent_store() {
    let reg = Registry::open_in_memory().unwrap();
    seed_task(&reg, "t");
    add_card(&reg, "c1", "t", None, "SUCCESS", 1.0).unwrap();
    add_card(&reg, "c2", "t", Some("c1"), "SUCCESS", 2.0).unwrap();
    add_card(&reg, "c3", "t", Some("c2"), "SUCCESS", 3.0).unwrap();
    add_card(&reg, "c4", "t", Some("c3"), "RUNTIME_ERROR", 0.0).unwrap();

    assert_eq!(reg.prune_cards("t", 1).unwrap(), 3);
    let left = reg.list_cards("t", None, None, 10).unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].card_id, "c3");
    assert_eq!(left[0].parent_a_id, None);

    assert!(reg.delete_task("t").unwrap());
    assert!(!reg.delete_task("t").unwrap());
    let st = reg.status().unwrap();
    assert_eq!(
        (st.tasks_count, st.cards_count, st.frontier_entries_count),
        (0, 0, 0)
    );
}

#[test]
fn recompute_scales_past_the_old_card_limit_without_decoding_code() {
    let reg = Registry::open_in_memory().unwrap();
    seed_task(&reg, "t");
    let hp = UtilityHyperparams::default();
    let mut parent: Option<String> = None;
    for i in 0..300 {
        let id = format!("c{i:04}");
        #[allow(clippy::cast_precision_loss)]
        let score = 10.0 + i as f64;
        add_card(&reg, &id, "t", parent.as_deref(), "SUCCESS", score).unwrap();
        parent = Some(id);
    }
    reg.recompute_task_utilities("t", &hp).unwrap();
    let best = reg.get_frontier("t").unwrap();
    assert_eq!(best.len(), DEFAULT_FRONTIER_SIZE);
    assert_eq!(best[0].card.card_id, "c0299");
    assert_eq!(reg.status().unwrap().cards_count, 300);
}

#[test]
fn snapshot_roundtrip_keeps_cards_across_a_file() {
    let mut reg = Registry::open_in_memory().unwrap();
    seed_task(&reg, "t");
    add_card(&reg, "c1", "t", None, "SUCCESS", 3.0).unwrap();
    let bytes = reg.snapshot_bytes().unwrap();
    let dir = std::env::temp_dir().join(format!("rsicraft-store-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("roundtrip.db");
    reg.restore_bytes(&bytes, Some(path.clone())).unwrap();
    assert_eq!(reg.db_path(), Some(path.as_path()));
    assert_eq!(reg.status().unwrap().cards_count, 1);
    reg.restore_bytes(&bytes, None).unwrap();
    assert!(reg.db_path().is_none());
    assert_eq!(reg.status().unwrap().cards_count, 1);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}
