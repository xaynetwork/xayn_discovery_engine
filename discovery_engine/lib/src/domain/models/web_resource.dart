/// [WebResource] class is used to represent different kinds of resources
/// like web, image, video, news, etc. that are delivered by an external
/// content API, which might serve search results, news, or other types.
class WebResource {
  final String title;
  final String snippet;
  final Uri url;
  final Uri displayUrl;

  const WebResource._({
    required this.title,
    required this.snippet,
    required this.url,
    required this.displayUrl,
  });
}
