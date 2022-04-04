## Mock (sm)bert model generator

Simple python script to generate dummy kpe (bert) and smbert ONNX models. It creates graph definition with the same inputs and outputs as original v0001 models.

# 📌 Prerequisites
You need to have [Python](https://www.python.org/) installed.

# 🏗 Usage

First install needed dependencies and then run the script specifying the type of the model (`kpe` or `smbert`) and output path. By default the output path is `"."` (the current directory).

```sh
# create virtual env
$ virtualenv .venv

# activate it
$ source .venv/bin/activate

# install dependencies
$ pip install -r requirements.txt

# generate kpe model and put it in the flutter example assets dir
$ python src/mock_onnx_tool.py \
  --type kpe \
  --output ../discovery_engine_flutter/example/assets/kpe_v0001

# generate smbert model and put it in the flutter example assets dir
$ python src/mock_onnx_tool.py \
  --type smbert \
  --output ../discovery_engine_flutter/example/assets/smbert_v0001
```
