# Current Xayn app functionalities

Below are listed functionalities available on each screen.

### Onboarding
- finish onboarding -> `DiscoveryScreen`

### MainScreen (DiscoveryScreen|SearchScreen|Webview)
- type into search input
  - open direct url (by typing valid url) -> `Webview`
  - trigger search (by typing new search query) -> `SearchScreen`
- open `Webview` from external link
- clear search input
- open nav menu
- go to collections screen -> `CollectionsScreen`
- go to settings screen -> `SettingsScreen`
- go to tabs screen -> `TabsScreen`
- create new tab -> `DiscoveryScreen`

### DiscoveryScreen
- fetch news items
- toggle personalisation
- news document
  - like / dislike / mark neutral
  - undo dislike
  - share
  - toggle bookmark
- scroll news list
- trigger deep search -> `SearchScreen`
- open news document (by tapping on a news document) -> `Webview`
- dismiss news categories / topics
- dismiss news sources

### SearchScreen
- fetch search results
- change search market
- toggle personalisation
- result document
  - like / dislike / mark neutral 
  - undo dislike
  - share
  - toggle bookmark
- scroll result list
- fetch next page
- trigger deep search
- trigger "new" search by changing search type
- trigger search from autosuggestions / prev queries
- open result document (by tapping) -> `Webview`
- navigate home -> `DiscoveryScreen`

### SearchHistory
- filter history entries by date (last 7 days, last month, etc.)
- remove all history under current filter
- remove single history entry
- open document -> `Webview`
- trigger search -> `SearchScreen`
- close `SearchHistory` -> `MainScreen`

#### Webview
- like / dislike / mark neutral page
- toggle reader mode
- navigate to new page by clicking a link on a current webview page
- scroll loaded page
- toggle bookmark
- reload page
- share page
- toggle blockers
- navigate home -> `DiscoveryScreen`
- close `Webview` -> `MainScreen`

### Tabs
- select current tab
- create new tab -> `DiscoveryScreen`
- remove tab
- remove all tabs
- close `TabsScreen` -> `MainScreen`

### Collections
- open collection details -> `BookmarksScreen`
- add new collection
- remove collection and related bookmarks
- remove collection but move related bookmarks to a different collection
- rename collection
- close `CollectionsScreen` -> `MainScreen`

### Bookmarks
- open bookmark -> `Webview`
- move bookmark to a different collection
- remove bookmark
- close `BookmarksScreen` -> `CollectionsScreen`

### SettingsScreen
- manage global setting for tracker blocking
- manage global setting for ad blocking
- manage global setting for cookie blocking
- clear data and cookies for all websites
- clear cache
- set Xayn as default browser
- change interface language
- change discovery screen layout
- change news market
- manage dismissed news topics / categories
- manage dismissed news sources
- change search market
- toggle default reader mode for news feed
- toggle default reader mode for other results
- visit info links -> `Webview`
- report a bug
- share the app
- close `SettingsScreen` -> `MainScreen`
