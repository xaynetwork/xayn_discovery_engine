// import the run_persona_based_benchmark function from the mind module
#[cfg(feature = "mind")]
use xayn_web_api::run_persona_based_benchmark;

#[cfg(feature = "mind")]
#[tokio::main]
async fn main() {
    if let Err(e) = run_persona_based_benchmark().await {
        eprintln!("{}", e);
    }
}
#[cfg(not(feature = "mind"))]
fn main() {}
