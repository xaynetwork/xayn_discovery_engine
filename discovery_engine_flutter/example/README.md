Example of using Discovery Engine in Flutter
============================================

This is a simple example of using the Discovery Engine in mobile Flutter app. The code is located in `lib/main.dart` file.

# 📌 Prerequisites
You need to have [Flutter](https://docs.flutter.dev/get-started/install) installed.

# 🏗 Usage

To run the example you need to start either iOS Simulator or Android Emulator app and execute the following in your terminal:

```sh
# from the root of the repo switch to `discovery_engine` dir
$ cd discovery_engine

# install dependencies
$ dart pub get

# run code generation
$ dart run build_runner build --delete-conflicting-outputs

# move to flutter plugin dir
$ cd ../discovery_engine_flutter

# install dependencies (this should install them in both, the plugin and the example)
$ flutter pub get

# switch to `example` dir
$ cd example

# run the example, this executes the code in bin/example.dart
$ flutter run
```

Happy discovering!
