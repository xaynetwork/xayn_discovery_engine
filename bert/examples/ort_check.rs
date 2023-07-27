use std::error;
use std::io::{self, Write};

use ndarray::{array, concatenate, s, Array1, Axis, CowArray};
use ort::{download::language::machine_comprehension::GPT2, tensor::OrtOwnedTensor, Environment, ExecutionProvider, GraphOptimizationLevel, OrtResult, SessionBuilder, Value, LoggingLevel};
use rand::Rng;
use ndarray::{Array2, ArrayBase, Ix2, OwnedRepr};
use serde::de::Error;
use xayn_ai_bert::{tokenizer::Tokenizer, Config, FirstPooler};
use xayn_test_utils::asset::{smbert, sts_data, zdf_data};
use tokenizers::{
    tokenizer::Tokenizer as HfTokenizer, PaddingDirection, PaddingParams, PaddingStrategy,
    TruncationDirection, TruncationParams, TruncationStrategy,
};


fn create_tokenizer(token_size: usize) -> Result<HfTokenizer, Box<dyn std::error::Error>> {
    let config = Config::new(smbert()?)?.with_token_size(token_size)?;
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

fn main() -> Result<(), Box<dyn error::Error>> {
    let token_size = 64;
    let tokenizer = create_tokenizer(token_size)?;
    // let token_reshaper =
    //     |slice: &[u32]| Array2::from_shape_fn((1, slice.len()), |(_, i)| i64::from(slice[i]));

    let environment = Environment::builder()
        .with_name("test")
        .with_log_level(LoggingLevel::Verbose)
        .build()?
        .into_arc();

    let mut session = SessionBuilder::new(&environment)?
        .with_model_from_file("/Users/maciejkrajewski/CLionProjects/xayn_discovery_engine/assets/smbert_v0004/model.onnx")?;

    let text = "This is a test sentence";
    let tokenizer_output = tokenizer.encode(text, false).unwrap();

    let tokens = tokenizer_output.get_ids().iter().map(|i| *i as i64).collect::<Vec<_>>();
    let mut tokens = CowArray::from(Array1::from_iter(tokens.iter().cloned()));
    let array_tokens = tokens.clone().insert_axis(Axis(0)).into_shape((1, tokens.shape()[0])).unwrap().into_dyn();
    let token_ids = Value::from_array(session.allocator(), &array_tokens)?;

    let attention_mask = tokenizer_output.get_attention_mask().iter().map(|i| *i as i64).collect::<Vec<_>>();
    let mut attention_mask = CowArray::from(Array1::from_iter(attention_mask.iter().cloned()));
    let array_attention = attention_mask.clone().insert_axis(Axis(0)).into_shape((1, attention_mask.shape()[0])).unwrap().into_dyn();
    let attention_mask = Value::from_array(session.allocator(), &array_attention)?;

    let token_type_ids = tokenizer_output.get_type_ids().iter().map(|i| *i as i64).collect::<Vec<_>>();
    let mut token_type_ids = CowArray::from(Array1::from_iter(token_type_ids.iter().cloned()));
    let array_types = token_type_ids.clone().insert_axis(Axis(0)).into_shape((1, token_type_ids.shape()[0])).unwrap().into_dyn();
    let token_type_ids = Value::from_array(session.allocator(), &array_types)?;

    let in_tensor = vec![token_ids, attention_mask, token_type_ids];
    let outputs = session.run(in_tensor)?;
    Ok(())
}

// fn main2() -> OrtResult<()> {
//     const PROMPT: &str = "The corsac fox (Vulpes corsac), also known simply as a corsac, is a medium-sized fox found in";
//     const GEN_TOKENS: i32 = 90;
//     const TOP_K: usize = 5;
//
//     let mut stdout = io::stdout();
//     let mut rng = rand::thread_rng();
//
//     let environment = Environment::builder()
//         .with_name("GPT-2")
//         .with_execution_providers([ExecutionProvider::CUDA(Default::default())])
//         .build()?
//         .into_arc();
//
//     // let session = SessionBuilder::new(&environment)?
//     //     .with_optimization_level(GraphOptimizationLevel::Level1)?
//     //     .with_intra_threads(1)?
//     //     .with_model_downloaded(GPT2::GPT2LmHead)?;
//
//     let tokenizer = tokenizers::Tokenizer::from_file("tests/data/gpt2-tokenizer.json").unwrap();
//     let tokens = tokenizer.encode(PROMPT, false).unwrap();
//     let tokens = tokens.get_ids().iter().map(|i| *i as i64).collect::<Vec<_>>();
//
//     let mut tokens = CowArray::from(Array1::from_iter(tokens.iter().cloned()));
//
//     print!("{PROMPT}");
//     stdout.flush().unwrap();
//
//     for _ in 0..GEN_TOKENS {
//         let n_tokens = tokens.shape()[0];
//         let array = tokens.clone().insert_axis(Axis(0)).into_shape((1, 1, n_tokens)).unwrap().into_dyn();
//         let inputs = vec![Value::from_array(session.allocator(), &array)?];
//         let outputs: Vec<Value> = session.run(inputs)?;
//         let generated_tokens: OrtOwnedTensor<f32, _> = outputs[0].try_extract()?;
//         let generated_tokens = generated_tokens.view();
//
//         let probabilities = &mut generated_tokens
//             .slice(s![0, 0, -1, ..])
//             .insert_axis(Axis(0))
//             .to_owned()
//             .iter()
//             .cloned()
//             .enumerate()
//             .collect::<Vec<_>>();
//         probabilities.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Less));
//
//         let token = probabilities[rng.gen_range(0..=TOP_K)].0;
//         tokens = CowArray::from(concatenate![Axis(0), tokens, array![token.try_into().unwrap()]]);
//
//         let token_str = tokenizer.decode(vec![token as _], true).unwrap();
//         print!("{}", token_str);
//         stdout.flush().unwrap();
//     }
//
//     println!();
//
//     Ok(())
// }