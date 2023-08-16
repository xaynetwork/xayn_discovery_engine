import os
from argparse import ArgumentParser
import onnx
from onnx import helper, TensorProto, GraphProto

def create_mock_onnx_model(model_path: str, graph_def: GraphProto) -> None:
    '''
    Create a mock onnx model.
    '''
    # Create the model (ModelProto)
    model_def = helper.make_model(
        graph_def,
        producer_name='com.xayn',
        opset_imports=[helper.make_opsetid('', 12)],
    )

    onnx.checker.check_model(model_def)
    print(f"The model is checked: \N{heavy check mark}")

    onnx.save(model_def, model_path)
    print(f"The model is saved under {model_path}: \N{heavy check mark}")

def create_bert_graph(embedding: int) -> GraphProto:
    # Create inputs (ValueInfoProto)
    input_type_proto = helper.make_tensor_type_proto(
        TensorProto.INT64,
        ['batch', 'sequence'],
    )
    inputs = [
        helper.make_value_info('input_ids', input_type_proto),
        helper.make_value_info('attention_mask', input_type_proto),
        helper.make_value_info('token_type_ids', input_type_proto),
    ]

    # Create outputs (ValueInfoProto)
    outputs = [
        helper.make_tensor_value_info(
            'output_0',
            TensorProto.FLOAT,
            ['batch', 'sequence', embedding],
        ),
        helper.make_tensor_value_info(
            'output_1',
            TensorProto.FLOAT,
            ['batch', embedding],
        ),
    ]

    # Create nodes (NodeProto)
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
            value=helper.make_tensor(
                name='const_tensor',
                data_type=TensorProto.INT64,
                dims=[1],
                vals=[embedding],
            )
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
    return helper.make_graph(
        nodes,
        'bert-mocked',
        inputs,
        outputs,
        [condition_tensor],
    )

def verify(model_path: str):
    '''
    Verify the model.
    '''
    import onnxruntime as rt

    print(f"Checking ONNX model loading from: {model_path} ...")
    try:
        sess = rt.InferenceSession(model_path, providers=rt.get_available_providers())
        for inp in sess.get_inputs():
            print("  input name='{}'\n    shape={}\n    type={}".format(inp.name, inp.shape, inp.type))
        for out in sess.get_outputs():
            print("  output name='{}'\n    shape={}\n    type={}".format(out.name, out.shape, out.type))
        print(f"Model {model_path} correctly loaded: \N{heavy check mark}")
    except Exception as re:
        print(f"Error while loading the model {re}: \N{heavy ballot x}")

if __name__ == '__main__':
    parser = ArgumentParser()
    parser.add_argument(
        "--output",
        type=str,
        required=False,
        help="Mock model's output path (ex: ../example/assets).",
    )
    parser.add_argument(
        "--type",
        choices=["bert"],
        required=False,
        default="bert",
        help="Type of the model.",
    )
    parser.add_argument(
        "--embedding",
        type=int,
        required=False,
        default=128,
        help="Embedding size of the model.",
    )

    args = parser.parse_args()
    model_path = os.path.abspath(args.output or os.path.curdir)

    if not os.path.isdir(model_path):
        print(f"The specified path: \"{args.output}\" does not exist: \N{heavy ballot x}")
        exit(1)

    model_path = os.path.join(model_path, "model.onnx")
    create_graph_choices = {
        "bert": create_bert_graph,
    }

    try:
        print("\n====== Converting model to ONNX ======")
        graph_def = create_graph_choices.get(args.type)(args.embedding)
        create_mock_onnx_model(model_path, graph_def)
        verify(model_path)
    except Exception as e:
        print(f"Error while converting the model: {e}")
        exit(1)
