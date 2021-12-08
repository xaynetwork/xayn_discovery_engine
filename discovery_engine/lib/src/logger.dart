import 'package:logger/logger.dart' show Logger;

final _defaultLogger = Logger();
Logger? _logger;
Logger get logger => _logger ?? _defaultLogger;

/// Initializes a global logger.
void initLogger(Logger logger) {
  _logger = logger;
}
