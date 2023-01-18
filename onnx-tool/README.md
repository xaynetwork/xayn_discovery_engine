## Mock Bert model generator

Simple python script to generate dummy bert ONNX models. It creates graph definition with the same inputs and outputs as original models.

# 📌 Prerequisites
You need to have [Python](https://www.python.org/) installed.

# 🏗 Usage

First install needed dependencies and then run the script specifying the type of the model (`smbert`) and output path. By default the output path is `"."` (the current directory).

```sh
# create virtual env
$ virtualenv .venv

# activate it
$ source .venv/bin/activate

# install dependencies
$ pip install -r requirements.txt

# generate smbert model, put it in the assets dir and upload a new version
$ python src/mock_onnx_tool.py \
  --type smbert \
  --output ../assets/smbert_mocked_v0004
```
