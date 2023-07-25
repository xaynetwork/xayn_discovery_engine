use std::time::Duration;
use derive_more::Deref;
use ndarray::Array2;
use xayn_ai_bert::{tokenizer::Tokenizer, Config, FirstPooler};
use onnxruntime::{environment::Environment, GraphOptimizationLevel, LoggingLevel};
use onnxruntime::tensor::OrtOwnedTensor;
use tokenizers::{PaddingDirection, PaddingParams, PaddingStrategy, tokenizer::Tokenizer as HfTokenizer, TruncationDirection, TruncationParams, TruncationStrategy};
use serde::{Deserialize, Deserializer};
use xayn_test_utils::asset::{sts_data, zdf_data, smbert};

fn create_tokenizer(token_size: usize) -> Result<HfTokenizer, Box<dyn std::error::Error>> {
    let config = Config::new(smbert()?)?
        .with_token_size(token_size)?;
    let mut tokenizer = HfTokenizer::from_file(config.dir.join("tokenizer.json")).unwrap();
    let padding_token = config.extract::<String>("tokenizer.tokens.padding")?;
    let padding = PaddingParams {
        strategy: PaddingStrategy::Fixed(token_size),
        direction: PaddingDirection::Right,
        pad_to_multiple_of: None,
        pad_id: tokenizer
            .token_to_id(&padding_token)
            .ok_or("missing padding token")?,
        pad_type_id: 0,
        pad_token: padding_token,
    };
    let truncation = TruncationParams {
        direction: TruncationDirection::Right,
        max_length: token_size,
        strategy: TruncationStrategy::LongestFirst,
        stride: 0,
    };
    tokenizer.with_padding(Some(padding));
    tokenizer.with_truncation(Some(truncation));

    Ok(tokenizer)
}

fn run_onnx(texts: Texts, token_size: usize) -> Result<Duration, Box<dyn std::error::Error>>{
    let tokenizer = create_tokenizer(token_size)?;
    let token_reshaper = |slice: &[u32]| Array2::from_shape_fn((1, slice.len()), |(_, i)| i64::from(slice[i]));

    let environment = Environment::builder()
        .with_name("test")
        .with_log_level(LoggingLevel::Verbose)
        .build()?;

    let mut session = environment
        .new_session_builder()?
        .with_optimization_level(GraphOptimizationLevel::Extended)?
        .with_number_threads(1)?
        .with_model_from_file("/Users/maciejkrajewski/CLionProjects/xayn_discovery_engine/assets/smbert_v0004/model.onnx")?;

    let start = std::time::Instant::now();
    for text in texts.into_vec() {
        let tokens = tokenizer.encode(text, false).unwrap();
        // reshape input tensor
        let token_ids = token_reshaper(tokens.get_ids());
        let attention_mask = token_reshaper(tokens.get_attention_mask());
        let type_ids = token_reshaper(tokens.get_type_ids());
        let in_tensor = vec![token_ids, attention_mask, type_ids];
        let outputs: Vec<OrtOwnedTensor<f32,_>> = session.run(in_tensor)?;
        //add average pooling at the end
    }
    Ok(start.elapsed())
}

fn run_tract(texts: Texts, token_size: usize) -> Result<Duration, Box<dyn std::error::Error>> {
    let pipeline = Config::new(smbert()?)?
        .with_token_size(token_size)?
        .with_tokenizer::<Tokenizer>()
        .with_pooler::<FirstPooler>()
        .build()?;

    let start = std::time::Instant::now();
    for text in texts.into_vec() {
        let embedding = pipeline.run(text)?;
    }
    Ok(start.elapsed())
}


#[derive(Debug)]
enum BenchmarkData {
    STS,
    ZDF,
}

#[derive(Debug)]
enum Embedder {
    ONNX,
    TRACT,
}

fn run_benchmark(benchmark_data: BenchmarkData, embedder_type: Embedder, token_size: usize) -> Result<(), Box<dyn std::error::Error>> {
    // print config of benchmark in single line
    println!("Benchmark: {:?}, Embedder: {:?}, Token size: {}", benchmark_data, embedder_type, token_size);
    let sentences = load_data(benchmark_data);
    // measure inference time
    let duration = match embedder_type {
        Embedder::ONNX => run_onnx(sentences, token_size)?,
        Embedder::TRACT => run_tract(sentences, token_size)?,
    };
    // print average inference time
    println!("Inference time: {:?}", duration);
    Ok(())
}

#[derive(Debug, Deref, Deserialize)]
#[serde(transparent)]
pub struct Texts(Vec<String>);

impl Texts {
    /// Reads json file with texts and returns a vector of texts.
    pub fn from_json(path: impl AsRef<std::path::Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
        let texts = serde_json::from_reader(file)?;
        Ok(Self(texts))
    }

    /// implement into so vector of texts can be used as input for tokenizer
    pub fn into_vec(self) -> Vec<String> {
        self.0
    }
}

fn load_data(benchmark_type: BenchmarkData) -> Texts {
    match benchmark_type {
        BenchmarkData::STS => Texts::from_json(sts_data().unwrap()).unwrap(),
        BenchmarkData::ZDF => Texts::from_json(zdf_data().unwrap()).unwrap(),
    }
}

/// run with `cargo run --example onnx_runtime`
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token_sizes = vec![64, 128, 256];
    // run benchmark for different token sizes
    for token_size in token_sizes {
        run_benchmark(BenchmarkData::STS, Embedder::TRACT, token_size)?;
        run_benchmark(BenchmarkData::STS, Embedder::ONNX, token_size)?;
        run_benchmark(BenchmarkData::ZDF, Embedder::TRACT, token_size)?;
        run_benchmark(BenchmarkData::ZDF, Embedder::ONNX, token_size)?;
    }
    Ok(())
}