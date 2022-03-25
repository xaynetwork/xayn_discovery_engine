import os
import onnx
from onnx import helper, TensorProto
import onnxruntime as rt


# Create one input (ValueInfoProto)
input_type_proto = helper.make_tensor_type_proto(
    TensorProto.INT64,
    ['batch', 'sequence'],
)
inputs = [
    helper.make_value_info('input_ids', input_type_proto),
    helper.make_value_info('attention_mask', input_type_proto),
    helper.make_value_info('token_type_ids', input_type_proto),
]

# Create one output (ValueInfoProto)
outputs = [
    helper.make_tensor_value_info(
        'output_0',
        TensorProto.FLOAT,
        ['batch', 'sequence', 'Addoutput_0_dim_2'],
    ),
    helper.make_tensor_value_info(
        'output_1',
        TensorProto.FLOAT,
        ['batch', 128],
    ),
]

# Create nodes (NodeProto)
const_tensor_value = helper.make_tensor(
    name='const_tensor',
    data_type=TensorProto.INT64,
    dims=[1],
    vals=[128],
)
condition_tensor = helper.make_tensor(
    'condition_tensor',
    TensorProto.BOOL,
    [2],
    [1, 0],
)
output_value = helper.make_tensor(
    'output_value',
    TensorProto.FLOAT,
    [1],
    [1],
)
nodes = [
    helper.make_node(
        'Constant',
        inputs=[],
        outputs=['const_tensor'],
        value=const_tensor_value
    ),
    helper.make_node(
        'Shape',
        inputs=['input_ids'],
        outputs=['input_ids_shape'],
    ),
    helper.make_node(
        'Concat',
        inputs=['input_ids_shape', 'const_tensor'],
        outputs=['output_0_shape'],
        axis=0,
    ),
    helper.make_node(
        'Compress',
        inputs=['input_ids_shape', 'condition_tensor'],
        outputs=['output_1_batch_shape'],
        axis=0,
    ),
    helper.make_node(
        'Concat',
        inputs=['output_1_batch_shape', 'const_tensor'],
        outputs=['output_1_shape'],
        axis=0,
    ),
    helper.make_node(
        'ConstantOfShape',
        inputs=['output_0_shape'],
        outputs=['output_0'],
        value=output_value,
    ),
    helper.make_node(
        'ConstantOfShape',
        inputs=['output_1_shape'],
        outputs=['output_1'],
        value=output_value,
    ),
]

# Create the graph (GraphProto)
graph_def = helper.make_graph(
    nodes,
    'test-model',
    inputs,
    outputs,
    [condition_tensor],
)

# Create the model (ModelProto)
model_def = helper.make_model(
    graph_def,
    producer_name='ai.onnx',
    opset_imports=[helper.make_opsetid('', 15)],
)

onnx.checker.check_model(model_def)
print('The model is checked!')

model_path = os.path.join('src', 'smbert-mocked.onnx')
model = onnx.save(model_def, model_path)
print('The model is saved!\n')

sess = rt.InferenceSession(model_path, providers=rt.get_available_providers())
for inp in sess.get_inputs():
    print("input name='{}' and shape={} and type={}".format(inp.name, inp.shape, inp.type))
for out in sess.get_outputs():
    print("output name='{}' and shape={} and type={}".format(out.name, out.shape, out.type))
