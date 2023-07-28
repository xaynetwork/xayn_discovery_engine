use std::error;
use std::io::{self, Write};

use ndarray::{array, concatenate, s, Array1, Axis, CowArray};
use ndarray::{Array2, ArrayBase, Ix2, OwnedRepr};
use ort::{
    download::language::machine_comprehension::GPT2, tensor::OrtOwnedTensor, Environment,
    ExecutionProvider, GraphOptimizationLevel, LoggingLevel, OrtResult, Session, SessionBuilder,
    Value,
};
use rand::Rng;
use serde::de::Error;
use tokenizers::{
    tokenizer::Tokenizer as HfTokenizer, PaddingDirection, PaddingParams, PaddingStrategy,
    TruncationDirection, TruncationParams, TruncationStrategy,
};
use xayn_ai_bert::tokenizer::Encoding;
use xayn_ai_bert::{tokenizer::Tokenizer, Config, FirstPooler};
use xayn_test_utils::asset::{smbert, sts_data, zdf_data};

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

    let mut session = SessionBuilder::new(&environment)?.with_model_from_file(
        "/Users/maciejkrajewski/CLionProjects/xayn_discovery_engine/assets/smbert_v0004/model.onnx",
    )?;

    let text = "This is a test sentence";
    let tokenizer_output = tokenizer.encode(text, false).unwrap();

    let tokens = tokenizer_output
        .get_ids()
        .iter()
        .map(|i| *i as i64)
        .collect::<Vec<_>>();
    let tokens = CowArray::from(Array1::from_iter(tokens.iter().cloned()));
    let array_tokens = tokens
        .clone()
        .insert_axis(Axis(0))
        .into_shape((1, tokens.shape()[0]))
        .unwrap()
        .into_dyn();

    let attention_mask = tokenizer_output
        .get_attention_mask()
        .iter()
        .map(|i| *i as i64)
        .collect::<Vec<_>>();
    let attention_mask = CowArray::from(Array1::from_iter(attention_mask.iter().cloned()));
    let array_attention = attention_mask
        .clone()
        .insert_axis(Axis(0))
        .into_shape((1, attention_mask.shape()[0]))
        .unwrap()
        .into_dyn();

    let token_type_ids = tokenizer_output
        .get_type_ids()
        .iter()
        .map(|i| *i as i64)
        .collect::<Vec<_>>();
    let token_type_ids = CowArray::from(Array1::from_iter(token_type_ids.iter().cloned()));
    let array_types = token_type_ids
        .clone()
        .insert_axis(Axis(0))
        .into_shape((1, token_type_ids.shape()[0]))
        .unwrap()
        .into_dyn();

    let in_tensor = vec![
        Value::from_array(session.allocator(), &array_tokens)?,
        Value::from_array(session.allocator(), &array_attention)?,
        Value::from_array(session.allocator(), &array_types)?,
    ];
    println!("before {:?}", in_tensor);
    let outputs = session.run(in_tensor)?;
    println!("after {:?}", outputs);
    Ok(())
}