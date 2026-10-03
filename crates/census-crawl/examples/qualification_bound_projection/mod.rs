mod binding;
mod collect;
mod input;
mod inspect;
mod io;
mod measure;
mod preserve;
mod summary;

use census_store::Store;
use serde_json::Value;

pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

struct Evidence {
    states: [measure::State; 7],
    attempts: [collect::Attempt; 3],
    binding: binding::Binding,
    projections: [Value; 2],
    captures: [Value; 3],
}

pub fn run() -> Result<()> {
    let args = input::arguments()?;
    let inputs = input::load(&args)?;
    preserve::initialize(&args.root, &inputs)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let evidence = execute(&args, &inputs, &runtime)?;
    let result = summary::summarize(&inputs, &evidence, input::unchanged(&args, &inputs)?)?;
    io::json(&args.root.join("qualification.json"), &result)?;
    std::fs::File::open(&args.root)?.sync_all()?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    if result.get("overall_success").and_then(Value::as_bool) != Some(true) {
        return Err("generic school-binding/replay requirements failed; complete observed evidence remains in qualification.json".into());
    }
    Ok(())
}

fn execute(
    args: &input::Arguments,
    inputs: &input::Inputs,
    runtime: &tokio::runtime::Runtime,
) -> Result<Evidence> {
    let store_root = args.root.join("store");
    let store = Store::open(&store_root)?;
    preserve::seed(&store, inputs)?;
    store.flush()?;
    let before = measure::save(&store, &args.root.join("00_before_unbound"), inputs)?;
    let first = runtime.block_on(collect::collect(&store))?;
    collect::save(&args.root, "unbound", &first)?;
    store.flush()?;
    let unbound = measure::save(&store, &args.root.join("01_after_unbound"), inputs)?;
    let first_capture = inspect::retained_capture(&store, inputs)?;
    drop(store);
    let store = Store::open(&store_root)?;
    let reopened_unbound = measure::save(&store, &args.root.join("02_reopened_unbound"), inputs)?;
    let binding = runtime.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(120),
            binding::derive(&store, inputs),
        )
        .await
    })??;
    binding::retain(&store, &args.root, &binding)?;
    store.flush()?;
    let bound_input = measure::save(&store, &args.root.join("03_before_bound_collect"), inputs)?;
    let second = runtime.block_on(collect::collect(&store))?;
    collect::save(&args.root, "bound", &second)?;
    store.flush()?;
    let bound = measure::save(&store, &args.root.join("04_after_bound"), inputs)?;
    let bound_projection = inspect::bound(&store, inputs, &binding)?;
    let second_capture = inspect::retained_capture(&store, inputs)?;
    drop(store);
    let store = Store::open(&store_root)?;
    let reopened_bound = measure::save(&store, &args.root.join("05_reopened_bound"), inputs)?;
    let third = runtime.block_on(collect::collect(&store))?;
    collect::save(&args.root, "replay", &third)?;
    store.flush()?;
    let replayed = measure::save(&store, &args.root.join("06_after_replay"), inputs)?;
    let replay_projection = inspect::bound(&store, inputs, &binding)?;
    let third_capture = inspect::retained_capture(&store, inputs)?;
    drop(store);
    Ok(Evidence {
        states: [
            before,
            unbound,
            reopened_unbound,
            bound_input,
            bound,
            reopened_bound,
            replayed,
        ],
        attempts: [first, second, third],
        binding,
        projections: [bound_projection, replay_projection],
        captures: [first_capture, second_capture, third_capture],
    })
}
