Example of using Discovery Engine
=================================

This is a simple example of using the Discovery Engine. The code is located in `lib/example.dart` file.

# 📌 Prerequisites
You need to have [Dart SDK](https://dart.dev/tools/sdk) installed.

# 🏗 Usage

To run the example you need to execute the following in your terminal:


## Preparation steps for both platforms (Web and VM)
```sh
# from the root of the repo switch to `discovery_engine` dir
$ cd discovery_engine

# install dependencies
$ dart pub get

# run code generation
$ dart run build_runner build --delete-conflicting-outputs

# switch to `example` dir
$ cd example

# install dependencies for the example
$ dart pub get
```

## Dart VM example

```sh
# run the example, this executes the code in bin/example.dart
$ dart run
```

## Web example

```sh
# activate the dart webdev package
dart pub global activate webdev

# compile the web worker file to javascript
dart compile js -o web/worker.dart.js ../lib/src/discovery_engine_worker.dart

# start the server
webdev serve

# open up you browser at http://localhost:8080, and check out the console
open http://localhost:8080
```

Happy discovering!
