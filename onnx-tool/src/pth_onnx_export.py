import torch
from torch import Tensor, nn
import onnx
import onnxruntime as rt
import numpy as np


class NeuralNetwork(nn.Module):
    def __init__(self):
        super(NeuralNetwork, self).__init__()
        self.flatten = nn.Flatten()
        self.linear = nn.Linear(64*3, 64*128 + 128)

    def forward(self, input_ids: Tensor, attention_mask: Tensor, token_type_ids: Tensor):
        batch, seq = input_ids.shape[:2]
        seq_x_2 = seq * 2

        x = self.flatten(input_ids.float())
        y = self.flatten(attention_mask.float())
        z = self.flatten(token_type_ids.float())

        input = torch.cat([x, y, z], dim=1)
        output_0_size = (batch * seq * seq_x_2)
        output_1_size = (batch * seq_x_2)

        output: Tensor = self.linear(input)
        output_0, output_1 = torch.split(
            output, [output_0_size, output_1_size], dim=1)

        output_0 = output_0.view(batch, seq, seq_x_2)
        output_1 = output_1.view(batch, seq_x_2)

        return output_0, output_1


model = NeuralNetwork()
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
