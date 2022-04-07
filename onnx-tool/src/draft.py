import os
from argparse import ArgumentParser
import onnx
from onnx import helper, TensorProto, GraphProto
import onnxruntime as rt
import numpy as np

def generate_model():
    input_type_proto = helper.make_tensor_type_proto(
        TensorProto.INT64,
        ['batch', 'sequence'],
    )
    inputs = [
        helper.make_value_info('input_ids', input_type_proto),
        helper.make_value_info('attention_mask', input_type_proto),
        helper.make_value_info('token_type_ids', input_type_proto),
    ]
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
    condition_tensor = helper.make_tensor(
        'condition_tensor',
        TensorProto.BOOL,
        [2],
        [1, 0],
    )
    nodes = [
        helper.make_node(
            'Constant',
            inputs=[],
            outputs=['const_128'],
            value=helper.make_tensor(
                name='const_128_tensor',
                data_type=TensorProto.INT64,
                dims=[1],
                vals=[128],
            )
        ),
        helper.make_node(
            'Constant',
            inputs=[],
            outputs=['axis'],
            value=helper.make_tensor(
                name='axis_tensor',
                data_type=TensorProto.INT64,
                dims=[1],
                vals=[2],
            )
        ),
        helper.make_node(
            'Shape',
            inputs=['input_ids'],
            outputs=['input_ids_shape'],
        ),
        helper.make_node(
            'Concat',
            inputs=['input_ids_shape', 'const_128'],
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
            inputs=['output_1_batch_shape', 'const_128'],
            outputs=['output_1_shape'],
            axis=0,
        ),
        helper.make_node(
           'Cast',
            inputs=['input_ids'],
            outputs=['input_ids_float'],
            to=TensorProto.FLOAT,
        ),
        helper.make_node(
            'Unsqueeze',
            inputs=['input_ids_float', 'axis'],
            outputs=['input_ids_float_plus_one'],
        ),
        helper.make_node(
            'Resize',
            inputs=['input_ids_float_plus_one', '', '', 'output_0_shape'],
            outputs=['output_0'],
            mode='linear',
        ),
        helper.make_node(
            'ConstantOfShape',
            inputs=['output_1_shape'],
            outputs=['output_1'],
            value=helper.make_tensor(
                'output_value',
                TensorProto.FLOAT,
                [1],
                [1],
            ),
        ),
    ]
    graph_def = helper.make_graph(
        nodes,
        'smbert-mocked',
        inputs,
        outputs,
        [condition_tensor],
    )
    model_def = helper.make_model(
        graph_def,
        producer_name='com.xayn',
        opset_imports=[helper.make_opsetid('', 15)],
    )

    model_path = os.path.join(os.path.curdir, "mocked.onnx")

    onnx.checker.check_model(model_def)
    print(f"The model is checked: \N{heavy check mark}")

    onnx.save(model_def, model_path)
    print(f"The model is saved under {model_path}: \N{heavy check mark}")

    print(f"Checking ONNX model loading from: {model_path} ...")

    sess = rt.InferenceSession(model_path, providers=rt.get_available_providers())
    for inp in sess.get_inputs():
        print("  input name='{}'\n    shape={}\n    type={}".format(inp.name, inp.shape, inp.type))
    for out in sess.get_outputs():
        print("  output name='{}'\n    shape={}\n    type={}".format(out.name, out.shape, out.type))
    print(f"Model {model_path} correctly loaded: \N{heavy check mark}")

    input_ids = np.random.random((1,64)).astype(np.int64)
    results = sess.run(
        ["output_0", "output_1"],
        {
            "input_ids": input_ids,
            "attention_mask": input_ids,
            "token_type_ids": input_ids,
        }
    )

    print(results[0].shape)
    print(results[0])
    return

if __name__ == '__main__':
    generate_model()
