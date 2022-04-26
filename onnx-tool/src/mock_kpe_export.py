import torch
from torch import Tensor, nn
import onnx
import onnxruntime as rt
import numpy as np


class MockKPEModel(nn.Module):
    def __init__(self):
        super(MockKPEModel, self).__init__()

    def forward(self, input_ids: Tensor, attention_mask: Tensor):
        embed_size = 768
        batch, seq = attention_mask.shape

        indices = input_ids.nonzero(as_tuple=True)[1]
        indices = indices.unsqueeze(0)

        output_0 = torch.gather(input_ids, 1, indices)
        output_0 = output_0.repeat(batch, seq, embed_size)
        output_0 = output_0[:, :, 0:embed_size].float()

        max = torch.max(output_0)
        std = torch.std(output_0)
        mean = torch.mean(output_0)
        output_0 = (output_0 - mean) / std
        last_hidden_state = output_0 + (mean / max)

        return last_hidden_state


model = MockKPEModel()

input_ids = torch.randint(10, (1, 64))
attention_mask = torch.randint(10, (1, 64))

model_path = "bert-mocked.onnx"
torch.onnx.export(
    model,
    (input_ids, attention_mask),
    model_path,
    opset_version=15,
    input_names=["input_ids", "attention_mask"],
    output_names=["last_hidden_state"],
    dynamic_axes={
        "input_ids": {0: "batch", 1: "sequence"},
        "attention_mask": {0: "batch", 1: "sequence"},
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
        ["last_hidden_state"],
        {
            "input_ids": input_ids,
            "attention_mask": input_ids,
        }
    )

    print(results[0].shape)
    # print(results[0])

except Exception as re:
    print(f"Error while loading the model {re}: \N{heavy ballot x}")
