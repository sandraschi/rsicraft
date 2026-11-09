use super::*;
use serde_json::json;

fn test_ctx() -> Ctx {
    let reg = Registry::open_in_memory().unwrap();
    Ctx::new(reg, None, None, None)
}

#[test]
fn operator_temperatures_and_code_extraction() {
    assert!((SearchOperator::Draft.target_temperature() - 0.70).abs() < 1e-6);
    assert!((SearchOperator::Improve.target_temperature() - 0.40).abs() < 1e-6);
    assert!((SearchOperator::Debug.target_temperature() - 0.20).abs() < 1e-6);
    assert!((SearchOperator::Crossover.target_temperature() - 0.50).abs() < 1e-6);

    let wrapped = "Here is the code:\n```python\ndef solve(x):\n    return x * 2\n```\nDone.";
    assert_eq!(
        PromptBuilder::extract_code_block(wrapped),
        "def solve(x):\n    return x * 2"
    );

    let thought_wrapped = "<|channel>thought\nLet's try ```python\nbad()\n``` first\n<channel|>```python\ndef solve(x):\n    return x + 1\n```";
    assert_eq!(
        PromptBuilder::extract_code_block(thought_wrapped),
        "def solve(x):\n    return x + 1"
    );
}

#[test]
fn sandbox_metric_parser_extracts_last_marker() {
    let logs = "Loading data...\nRSI_METRIC_JSON={\"status\":\"SUCCESS\",\"score\":91.25}\n";
    let (status, score) = SandboxRunner::parse_evaluator_output(logs);
    assert_eq!(status, "SUCCESS");
    assert!((score - 91.25).abs() < 1e-6);

    let bad_logs = "Segfault before metric output";
    let (bad_status, bad_score) = SandboxRunner::parse_evaluator_output(bad_logs);
    assert_eq!(bad_status, "METRIC_PARSE_ERROR");
    assert!((bad_score - 0.0).abs() < 1e-6);
}

#[test]
fn demo_seed_campaign_and_lineage_dag() {
    let mut ctx = test_ctx();
    let seed = dispatch(&mut ctx, Command::DemoSeed).unwrap();
    assert_eq!(seed["seeded"], true);
    assert_eq!(seed["cards_created"], 4);

    // Repeating demo_seed on non-empty DB is refused
    assert!(dispatch(&mut ctx, Command::DemoSeed).is_err());

    let frontier = dispatch(
        &mut ctx,
        Command::GetFrontier {
            task_id: "opt-synth".into(),
            limit: 10,
        },
    )
    .unwrap();
    let arr = frontier["frontier"].as_array().unwrap();
    assert_eq!(arr.len(), 4);
    assert!(arr[0]["primary_score"].as_f64().unwrap() >= arr[1]["primary_score"].as_f64().unwrap());

    let lineage = dispatch(
        &mut ctx,
        Command::Lineage {
            task_id: "opt-synth".into(),
        },
    )
    .unwrap();
    assert_eq!(lineage["nodes_count"], 4);
    assert!(lineage["edges_count"].as_u64().unwrap() >= 3);
}

#[test]
fn direct_evaluate_detects_compile_and_runtime_errors() {
    let mut ctx = test_ctx();
    dispatch(
        &mut ctx,
        Command::TaskUpsert {
            task_id: "t1".into(),
            name: "Test".into(),
            description: "Write solve(x)".into(),
            evaluator_script: "print('RSI_METRIC_JSON={\"status\":\"SUCCESS\",\"score\":1.0}')"
                .into(),
        },
    )
    .unwrap();

    let bad = Command::from_tool(
        "rsi_evaluate",
        &json!({
            "task_id": "t1",
            "code": "not valid function",
            "record": false
        }),
    )
    .unwrap();
    let bad_res = dispatch(&mut ctx, bad).unwrap();
    assert_eq!(bad_res["status"], "COMPILE_ERROR");

    let crash = Command::from_tool(
        "rsi_evaluate",
        &json!({
            "task_id": "t1",
            "code": "def solve(x):\n    return 1 / 0",
            "record": true
        }),
    )
    .unwrap();
    let crash_res = dispatch(&mut ctx, crash).unwrap();
    assert_eq!(crash_res["execution_status"], "RUNTIME_ERROR");
}

#[test]
fn multitab_help_supports_all_tabs_and_substring_search() {
    let mut ctx = test_ctx();
    let all = dispatch(
        &mut ctx,
        Command::Help {
            tab: None,
            search: None,
        },
    )
    .unwrap();
    assert_eq!(all["matched_tabs_count"], 9);

    let tsinghua = dispatch(
        &mut ctx,
        Command::Help {
            tab: Some("tsinghua_openrsi".into()),
            search: None,
        },
    )
    .unwrap();
    assert_eq!(tsinghua["matched_tabs_count"], 1);
    assert_eq!(tsinghua["tabs"][0]["id"], "tsinghua_openrsi");

    let filtered = dispatch(
        &mut ctx,
        Command::Help {
            tab: None,
            search: Some("Kadane".into()),
        },
    )
    .unwrap();
    assert!(filtered["matched_tabs_count"].as_u64().unwrap() >= 1);

    let bad = dispatch(
        &mut ctx,
        Command::Help {
            tab: Some("nonexistent".into()),
            search: None,
        },
    );
    assert!(bad.is_err());
}
