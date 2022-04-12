import torch
from torch import Tensor, nn
import onnx
import onnxruntime as rt
import numpy as np


class MockOnnxModel(nn.Module):
    def __init__(self):
        super(MockOnnxModel, self).__init__()

    def forward(self, input_ids: Tensor, attention_mask: Tensor, token_type_ids: Tensor):
        const_3 = torch.ones_like(attention_mask) * 3
        non_zero = torch.where(attention_mask == 0, const_3, attention_mask)

        rand = input_ids / non_zero
        rand = torch.cat([rand, rand], dim=1)
        rand = rand.transpose(0, 1).float()

        output_0 = torch.mul(input_ids, rand)
        output_0 = torch.unsqueeze(output_0, 0)
        output_0 = output_0.transpose(1, 2)
        mean = torch.mean(output_0)
        std = torch.std(output_0)
        output_0 = (output_0 - mean) / std

        output_1 = torch.mul(token_type_ids, rand)
        output_1 = output_1.transpose(0, 1)
        output_1 = output_1[:1, :]

        return output_0, output_1


model = MockOnnxModel()

input_ids = torch.randint(10, (1, 64))
attention_mask = torch.randint(10, (1, 64))
token_type_ids = torch.randint(10, (1, 64))

model_path = "mocked.onnx"
torch.onnx.export(
    model,
    (input_ids, attention_mask, token_type_ids),
    model_path,
    opset_version=15,
    input_names=["input_ids", "attention_mask", "token_type_ids"],
    output_names=["output_0", "output_1"],
    dynamic_axes={
        "input_ids": {0: "batch", 1: "sequence"},
        "attention_mask": {0: "batch", 1: "sequence"},
        "token_type_ids": {0: "batch", 1: "sequence"},
    }
)

# Load the ONNX model
model = onnx.load(model_path)

# Check that the model is well formed
onnx.checker.check_model(model)

print(f"Checking ONNX model loading from: {model_path} ...")
try:
    sess = rt.InferenceSession(
        model_path, providers=rt.get_available_providers())

    input_ids = np.random.randint(1, 120, (1, 64))
    results = sess.run(
        ["output_0", "output_1"],
        {
            "input_ids": input_ids,
            "attention_mask": input_ids,
            "token_type_ids": input_ids,
        }
    )

    print(results[0].shape)
    # print(results[0])
    print(results[1].shape)
    # print(results[1])

except Exception as re:
    print(f"Error while loading the model {re}: \N{heavy ballot x}")
