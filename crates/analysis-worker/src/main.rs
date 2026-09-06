//! Isolated worker. Sample bytes arrive on stdin frames. They are never executed.

use std::io::{self, BufReader, Write};

use unveil_analysis_worker::{dispatch, read_body};
use unveil_contracts::{read_frame, write_frame, WorkerRequest};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--isolation-self-test-child") {
        std::process::exit(2);
    }
    if !args.iter().any(|a| a == "--isolated") {
        eprintln!("unveil-worker: refuse to run without coordinator isolation (--isolated)");
        std::process::exit(1);
    }

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let header = match read_frame(&mut reader, 1024 * 1024) {
        Ok(h) => h,
        Err(err) => {
            eprintln!("unveil-worker: header: {err}");
            std::process::exit(1);
        }
    };
    let req: WorkerRequest = match serde_json::from_slice(&header) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("unveil-worker: schema");
            std::process::exit(1);
        }
    };
    let body = if req.op == "self_test" {
        Vec::new()
    } else {
        match read_body(&mut reader) {
            Ok(b) => b,
            Err(err) => {
                eprintln!("unveil-worker: body: {err}");
                std::process::exit(1);
            }
        }
    };
    let resp = dispatch(&req, &body);
    let json = serde_json::to_vec(&resp).unwrap_or_else(|_| b"{\"ok\":false}".to_vec());
    let mut stdout = io::stdout().lock();
    if write_frame(&mut stdout, &json).is_err() {
        std::process::exit(1);
    }
    let _ = stdout.flush();
}
