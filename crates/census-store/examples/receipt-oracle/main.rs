use serde_json::json;
use std::fs;

mod args;
mod checks;
mod report;
mod rows;
mod verify;
mod witness;

use args::parse_args;

fn main() {
    let args = match parse_args(std::env::args().skip(1).collect()) {
        Ok(args) => args,
        Err(fault) => {
            eprintln!("USAGE: {}", fault.message);
            std::process::exit(fault.code);
        }
    };
    let mut log = Vec::new();
    match verify::verify(&args, &mut log) {
        Ok(verdict) => {
            let text = log.join("\n");
            println!("{text}");
            println!(
                "PASS: {} exact physical conservation verified (rows={}, receipts={}, seq={}, copy={})",
                args.kind,
                verdict.rows_marker,
                verdict.receipts_marker,
                verdict.sequence_marker,
                verdict.copy_dir.display()
            );
            drop(fs::write(args.evidence.join("receipt-oracle.log"), text));
        }
        Err(fault) => {
            log.push(format!(
                "{}: {}",
                if fault.code == 1 { "FAIL" } else { "USAGE" },
                fault.message
            ));
            let text = log.join("\n");
            println!("{text}");
            drop(fs::write(args.evidence.join("receipt-oracle.log"), &text));
            let report = serde_json::to_vec_pretty(&json!({
                "verdict": "fail",
                "kind": args.kind,
                "operation": args.operation,
                "invocation": args.invocation,
                "invocation_bound": false,
                "message": fault.message,
            }))
            .map_or(Vec::new(), std::convert::identity);
            drop(fs::write(args.evidence.join("receipt-oracle.json"), report));
            std::process::exit(fault.code);
        }
    }
}
