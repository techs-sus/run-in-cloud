# run-in-cloud

run-in-cloud is a [run-in-roblox](https://github.com/rojo-rbx/run-in-roblox) replacement ment to execute Luau code via a [LuauExecutionSessionTask](https://create.roblox.com/docs/cloud/reference/LuauExecutionSessionTask). It prints the logs out, just like [run-in-roblox](https://github.com/rojo-rbx/run-in-roblox) does.

## Differences from run-in-roblox

- all code is run via [LuauExecutionSessionTask's](https://create.roblox.com/docs/cloud/reference/LuauExecutionSessionTask) meaning you can use `Script.Source`, `ModuleScript.Source` and anything which is [`🔐PluginOrOpenCloud`](https://raw.githubusercontent.com/MaximumADHD/Roblox-Client-Tracker/refs/heads/roblox/API-Dump.txt); however you cannot use PluginSecurity like in [run-in-roblox](https://github.com/rojo-rbx/run-in-roblox)
- you must login with a Open Cloud API key with scopes `universe-places:write`, `universe.place.luau-execution-session:read`, `universe.place.luau-execution-session:write` and sufficent IP allowlists; login is done via `run-in-cloud login --key apiKey --universe-id universeId --place-id placeId`
- the syntax is slightly different, you need to call the subcommand run, as shown: `run-in-cloud run --place place.rbxl --script script.luau`
