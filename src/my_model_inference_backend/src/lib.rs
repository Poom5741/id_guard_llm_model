mod storage;

use prost::Message;
use ic_cdk::println;
use std::cell::RefCell;
use anyhow::anyhow;
use tract_onnx::prelude::*;
use tract_ndarray::{ArrayD, IxDyn};

// Import necessary types from Tract's prelude
use tract_onnx::prelude::{tvec, TVec};

pub const MODEL_FILE: &str = "qwen2_5_1_5B_instruct.onnx";

// Define model type
type Model = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

thread_local! {
    static MODEL: RefCell<Option<Model>> = RefCell::new(None);
}

// Constructs a runnable model from the serialized ONNX model.
pub fn setup() -> TractResult<()> {
    let bytes = storage::bytes(MODEL_FILE);
    let proto = tract_onnx::pb::ModelProto::decode(bytes)
        .map_err(|e| anyhow!("Failed to decode model proto: {}", e))?;

    let model = tract_onnx::onnx()
        .model_for_proto_model(&proto)?
        .into_optimized()?
        .into_runnable()?;

    MODEL.with(|m| {
        *m.borrow_mut() = Some(model);
    });

    Ok(())
}

#[ic_cdk::update]
fn setup_model() -> Result<(), String> {
    setup().map_err(|err| format!("Failed to setup model: {}", err))
}

#[ic_cdk::update]
fn model_inference(max_tokens: u8, numbers: Vec<i64>) -> Result<Vec<i64>, String> {
    create_tensor_and_run_model(max_tokens, numbers).map_err(|err| err.to_string())
}

// Runs the model on the given token_ids and returns generated tokens.
pub fn create_tensor_and_run_model(max_tokens: u8, token_ids: Vec<i64>) -> Result<Vec<i64>, anyhow::Error> {
    MODEL.with(|model| {
        let model = model.borrow();
        let model = model.as_ref().unwrap();

        let mut past_key_values_tensor = create_empty_past_key_values(24, 1, 12, 0, 64)?;

        let mut input_ids = token_ids;
        let mut attention_mask: Vec<i8> = vec![1; input_ids.len()];
        let mut output_ids: Vec<i64> = Vec::new();

        for _ in 0..max_tokens {
            let input_ids_tensor = create_tensor_i64(&input_ids)?;
            let attention_mask_tensor = create_tensor_i8(&attention_mask)?;

            let inputs: TVec<Tensor> = tvec!(
                input_ids_tensor.into(),
                attention_mask_tensor.into(),
                past_key_values_tensor.clone().into()
            );

            let outputs = model.run(inputs)?;

            let next_token_tensor = outputs[0].to_array_view::<i64>()?;
            let next_token = next_token_tensor[[0, 0]];

            past_key_values_tensor = outputs[1].clone().into_tensor();

            println!("Next token: {}", next_token);
            if next_token == 50256_i64 {
                break;
            }

            input_ids = vec![next_token];
            attention_mask.push(1);
            output_ids.push(next_token);
        }

        Ok(output_ids)
    })
}

fn create_tensor_i64(data: &[i64]) -> TractResult<Tensor> {
    let shape = [1, data.len()];
    let array = ArrayD::from_shape_vec(IxDyn(&shape), data.to_vec())
        .map_err(|_| anyhow!("Failed to create tensor from shape and values"))?;
    Ok(array.into_tensor())
}

fn create_tensor_i8(data: &[i8]) -> TractResult<Tensor> {
    let shape = [1, data.len()];
    let array = tract_ndarray::Array::from_shape_vec(shape, data.to_vec())
        .map_err(|_| anyhow::anyhow!("Failed to create tensor from shape and values"))?;
    Ok(array.into_tensor())
}

fn create_empty_past_key_values(
    num_layers: usize,
    batch_size: usize,
    num_heads: usize,
    seq_length: usize,
    head_dim: usize,
) -> TractResult<Tensor> {
    let shape = [num_layers, batch_size, num_heads, seq_length, head_dim];
    let array = tract_ndarray::Array::from_shape_vec(
        IxDyn(&shape),
        vec![0.0_f32; num_layers * batch_size * num_heads * seq_length * head_dim],
    )
    .map_err(|_| anyhow!("Failed to create tensor from shape and values"))?;
    Ok(array.into_tensor())
}