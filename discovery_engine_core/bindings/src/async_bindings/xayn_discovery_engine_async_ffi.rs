#! [doc(hidden)] #!
[allow(clippy :: unused_unit, clippy :: semicolon_if_nothing_returned, clippy
:: used_underscore_binding, clippy :: module_name_repetitions,
unreachable_pub,)] use xayn_discovery_engine_ai :: Embedding ; use
xayn_discovery_engine_core ::
{
    document ::
    { Document, HistoricDocument, TimeSpent, TrendingTopic, UserReacted },
    InitConfig,
} ; use xayn_discovery_engine_providers :: Market ; use crate :: types ::
engine :: SharedEngine ; pub struct XaynDiscoveryEngineAsyncFfi ;
#[doc = r" Initializes the dart api."] #[doc = r""]
#[doc = r" It's safe to be called multiple times and from multiple threads."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc = r" Must be called with a pointer produced by dart using"]
#[doc = r" `NativeApi.initializeApiDLData`."] #[no_mangle] pub unsafe extern
"C" fn
async_bindgen_dart_init_api__xayn_discovery_engine_async_ffi(init_data : * mut
:: std :: ffi :: c_void) -> u8
{
    unsafe
    {
        :: async_bindgen :: dart ::
        initialize_dart_api_dl(init_data).is_ok().into()
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__initialize(config :
Box < InitConfig >, state : Option < Box < Vec < u8 > > >, history : Box < Vec
< HistoricDocument > >, async_bindgen_dart_port_id : i64,
async_bindgen_dart_completer_id : i64) -> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                initialize(config, state, history))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__initialize(handle
: i64) -> Box < Result < SharedEngine, String > >
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__serialize(engine : &
SharedEngine, async_bindgen_dart_port_id : i64,
async_bindgen_dart_completer_id : i64) -> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                serialize(engine))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__serialize(handle :
i64) -> Box < Result < Vec < u8 >, String > >
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__set_markets(engine :
& SharedEngine, markets : Box < Vec < Market > >, history : Box < Vec <
HistoricDocument > >, async_bindgen_dart_port_id : i64,
async_bindgen_dart_completer_id : i64) -> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                set_markets(engine, markets, history))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__set_markets(handle
: i64) -> Box < Result < (), String > >
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__get_feed_documents(engine
: & SharedEngine, history : Box < Vec < HistoricDocument > >, max_documents :
u32, async_bindgen_dart_port_id : i64, async_bindgen_dart_completer_id : i64)
-> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                get_feed_documents(engine, history, max_documents))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__get_feed_documents(handle
: i64) -> Box < Result < Vec < Document >, String > >
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__time_spent(engine :
& SharedEngine, time_spent : Box < TimeSpent >, async_bindgen_dart_port_id :
i64, async_bindgen_dart_completer_id : i64) -> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                time_spent(engine, time_spent))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__time_spent(handle
: i64) -> Box < Result < (), String > >
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__user_reacted(engine
: & SharedEngine, history : Option < Box < Vec < HistoricDocument > > >,
reacted : Box < UserReacted >, async_bindgen_dart_port_id : i64,
async_bindgen_dart_completer_id : i64) -> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                user_reacted(engine, history, reacted))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__user_reacted(handle
: i64) -> Box < Result < (), String > >
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__search_by_query(engine
: & SharedEngine, query : Box < String >, page : u32, page_size : u32,
async_bindgen_dart_port_id : i64, async_bindgen_dart_completer_id : i64) -> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                search_by_query(engine, query, page, page_size))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__search_by_query(handle
: i64) -> Box < Result < Vec < Document >, String > >
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__search_by_topic(engine
: & SharedEngine, topic : Box < String >, page : u32, page_size : u32,
async_bindgen_dart_port_id : i64, async_bindgen_dart_completer_id : i64) -> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                search_by_topic(engine, topic, page, page_size))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__search_by_topic(handle
: i64) -> Box < Result < Vec < Document >, String > >
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__deep_search(engine :
& SharedEngine, term : Box < String >, market : Box < Market >, embedding :
Box < Embedding >, async_bindgen_dart_port_id : i64,
async_bindgen_dart_completer_id : i64) -> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                deep_search(engine, term, market, embedding))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__deep_search(handle
: i64) -> Box < Result < Vec < Document >, String > >
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__trending_topics(engine
: & SharedEngine, async_bindgen_dart_port_id : i64,
async_bindgen_dart_completer_id : i64) -> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                trending_topics(engine))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__trending_topics(handle
: i64) -> Box < Result < Vec < TrendingTopic >, String > >
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__set_trusted_sources(engine
: & SharedEngine, history : Box < Vec < HistoricDocument > >, sources : Box <
Vec < String > >, async_bindgen_dart_port_id : i64,
async_bindgen_dart_completer_id : i64) -> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                set_trusted_sources(engine, history, sources))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__set_trusted_sources(handle
: i64) -> Box < Result < (), String > >
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__set_excluded_sources(engine
: & SharedEngine, history : Box < Vec < HistoricDocument > >, sources : Box <
Vec < String > >, async_bindgen_dart_port_id : i64,
async_bindgen_dart_completer_id : i64) -> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                set_excluded_sources(engine, history, sources))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__set_excluded_sources(handle
: i64) -> Box < Result < (), String > >
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__dispose(engine : Box
< SharedEngine >, async_bindgen_dart_port_id : i64,
async_bindgen_dart_completer_id : i64) -> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                dispose(engine))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__dispose(handle :
i64) -> ()
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
} #[doc = r" Wrapper for initiating the call to an async function."]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" The caller must make sure all lifetimes and aliasing constraints are valid until"]
#[doc = r" the rust-future completed or was dropped."] #[doc = r""]
#[doc =
r" This means non-owning references to extern allocated memory you pass in must"]
#[doc = r" not be:"] #[doc = r""] #[doc = r" - deallocated"]
#[doc = r" - accessed (if `&mut`)"] #[doc = r" - modified (if `&`)"]
#[doc = r""]
#[doc = r" See the `spawn` method documentation of the `PreparedCompleter`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_call__xayn_discovery_engine_async_ffi__reset_ai(engine : &
SharedEngine, async_bindgen_dart_port_id : i64,
async_bindgen_dart_completer_id : i64) -> u8
{
    match :: async_bindgen :: dart :: PreparedCompleter ::
    new(async_bindgen_dart_port_id, async_bindgen_dart_completer_id)
    {
        Ok(completer) =>
        {
            unsafe
            {
                completer.spawn(XaynDiscoveryEngineAsyncFfi ::
                reset_ai(engine))
            } ; 1
        } Err(_) => 0
    }
}
#[doc =
r#" Extern "C"  wrapper delegating to `PreparedCompleter::extract_result()`."#]
#[doc = r""] #[doc = r" # Safety"] #[doc = r""]
#[doc =
r" See the language specific version of `PreparedCompleter::extract_result()`."]
#[no_mangle] pub unsafe extern "C" fn
async_bindgen_dart_return__xayn_discovery_engine_async_ffi__reset_ai(handle :
i64) -> Box < Result < (), String > >
{
    unsafe
    {
        :: async_bindgen :: dart :: PreparedCompleter ::
        extract_result(handle)
    }
}