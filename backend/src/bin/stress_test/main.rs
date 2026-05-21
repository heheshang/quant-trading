use clap::Parser as ClapParser;
use std::path::PathBuf;

mod http_client;
mod reporter;
mod scenarios;
mod ws_client;

use http_client::HttpStressClient;
use reporter::ReportGenerator;
use scenarios::get_scenario;

#[derive(ClapParser, Debug)]
#[command(name = "stress-test")]
#[command(about = "Quant Trading Backend Stress Testing Tool")]
struct Args {
    /// Scenario name (order_write, order_read, portfolio, health)
    #[arg(long)]
    scenario: String,

    /// Number of concurrent workers
    #[arg(long, default_value = "10")]
    concurrency: usize,

    /// Test duration in seconds
    #[arg(long, default_value = "30")]
    duration: u64,

    /// Warmup duration in seconds
    #[arg(long, default_value = "5")]
    warmup: u64,

    /// Target host
    #[arg(long, default_value = "http://localhost:8080")]
    host: String,

    /// Auth token (optional)
    #[arg(long)]
    token: Option<String>,

    /// Output file path (JSON)
    #[arg(long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let scenario = get_scenario(&args.scenario).unwrap_or_else(|| {
        eprintln!("Unknown scenario: {}", args.scenario);
        std::process::exit(1);
    });

    println!(
        "🧪 Stress Test: {} | concurrency={} | duration={}s | warmup={}s",
        scenario.name, args.concurrency, args.duration, args.warmup
    );

    let scenario_name = scenario.name.clone();
    let scenario_type = scenario.scenario_type.clone();

    match scenario_type.as_str() {
        "http" => run_http_stress(args, *scenario).await,
        "ws" => run_ws_stress(args, *scenario).await,
        _ => {
            eprintln!("Unknown scenario type: {}", scenario_type);
            std::process::exit(1);
        }
    }
}

async fn run_http_stress(args: Args, scenario: scenarios::Scenario) -> Result<(), Box<dyn std::error::Error>> {
    let client = HttpStressClient::new(&args.host, args.token.as_deref()).await?;

    // Warmup phase
    println!("🔥 Warmup ({}s)...", args.warmup);
    let warmup_start = std::time::Instant::now();
    while warmup_start.elapsed().as_secs() < args.warmup {
        let _ = client
            .request(&scenario.method, &scenario.endpoint, scenario.body.as_deref())
            .await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Actual stress test
    println!("💥 Stress test ({}s)...", args.duration);
    let results = client
        .stress_test(
            &scenario.method,
            &scenario.endpoint,
            scenario.body,
            args.concurrency,
            args.duration,
        )
        .await?;

    // Generate report
    let report = ReportGenerator::generate_http_report(&scenario.name, &results);
    let json = serde_json::to_string_pretty(&report)?;

    if let Some(path) = &args.output {
        std::fs::write(path, &json)?;
        println!("📄 Report saved to: {}", path.display());
    } else {
        println!("{}", json);
    }

    // Print summary
    println!("\n📊 Summary:");
    println!("  Total requests: {}", results.total_requests);
    println!("  QPS: {:.2}", report.qps);
    println!("  P50: {}ms", results.p50_ms());
    println!("  P99: {}ms", results.p99_ms());
    println!("  Error rate: {:.2}%", report.error_rate * 100.0);

    if report.pass {
        println!("\n✅ PASS - All thresholds met");
    } else {
        println!("\n❌ FAIL - Some thresholds not met");
        std::process::exit(1);
    }

    Ok(())
}

async fn run_ws_stress(_args: Args, _scenario: scenarios::Scenario) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔌 WebSocket stress test not yet implemented");
    println!("   (Use HTTP scenarios for now: order_write, order_read, portfolio, health)");
    Ok(())
}
